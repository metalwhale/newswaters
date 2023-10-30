mod bert;
mod llama;

use std::env;

use anyhow::{Context, Error, Result};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use candle_transformers::models::jina_bert::BertModel;
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokenizers::Tokenizer;
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Clone)]
struct AppState {
    llama_model_path: String,
    llama_instruct_template: String,
    bert_model: BertModel, // TODO: Need a type that is more general
    bert_tokenizer: Tokenizer,
}

// See: https://github.com/tokio-rs/axum/blob/c979672/examples/anyhow-error-response/src/main.rs#L34-L57
struct AppError(Error);
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}
impl<E> From<E> for AppError
where
    E: Into<Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let model_dir = "/usr/src/models".to_string();
    let state = initialize(&model_dir).await?;
    println!("[INFO] main: state.llama_model_path={}", &state.llama_model_path);
    let app = Router::new()
        .route("/healthz", routing::get(|| async { "Ok" }))
        .route("/instruct", routing::post(instruct))
        .route("/embed", routing::post(embed))
        .with_state(state);
    let port = env::var("ECHOLOCATOR_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn initialize(model_dir: &str) -> Result<AppState> {
    // Llama
    let llama_model_url = env::var("ECHOLOCATOR_LLAMA_MODEL_URL").unwrap_or(
        "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.Q5_K_M.gguf".to_string(),
    );
    let llama_instruct_template =
        env::var("ECHOLOCATOR_LLAMA_INSTRUCT_TEMPLATE").unwrap_or("<s>[INST] {instruction} [/INST]".to_string());
    let llama_model_name = llama_model_url
        .split("/")
        .last()
        .context(format!("model_url={llama_model_url}"))?
        .to_string();
    let llama_model_path = format!("{model_dir}/{llama_model_name}");
    // Bert
    let bert_model_name =
        env::var("ECHOLOCATOR_BERT_MODEL_NAME").unwrap_or("jinaai/jina-embeddings-v2-base-en".to_string());
    let bert_tokenizer_name =
        env::var("ECHOLOCATOR_BERT_TOKENIZER_NAME").unwrap_or("sentence-transformers/all-MiniLM-L6-v2".to_string());
    let (bert_model, bert_tokenizer) = bert::build_model_and_tokenizer(bert_model_name, bert_tokenizer_name)?;
    // App state
    let state = AppState {
        llama_model_path,
        llama_instruct_template,
        bert_model,
        bert_tokenizer,
    };
    if std::path::Path::new(&state.llama_model_path).exists() {
        return Ok(state);
    }
    let mut file = File::create(&state.llama_model_path).await?;
    let mut stream = reqwest::get(llama_model_url).await?.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(state)
}

#[derive(Deserialize)]
struct InstructRequest {
    instruction: String,
}

#[derive(Serialize)]
struct InstructResponse {
    completion: String,
}

async fn instruct(
    State(state): State<AppState>,
    Json(payload): Json<InstructRequest>,
) -> Result<Json<InstructResponse>, AppError> {
    let prompt = state
        .llama_instruct_template
        .replace("{instruction}", &payload.instruction);
    let completion = llama::inference(&state.llama_model_path, &prompt).await?;
    let response = InstructResponse {
        completion: completion
            // TODO: Prevent the output of the prompt rather than having to manually remove it from the completion
            .replace(
                // prompt lacks both BOS and EOS markers in the completion
                &prompt.replace("<s>", "").replace("</s>", ""),
                "",
            )
            .trim()
            .to_string(),
    };
    Ok(Json(response))
}

#[derive(Deserialize)]
struct EmbedRequest {
    sentence: String,
}

#[derive(Serialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

async fn embed(
    State(state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, AppError> {
    let embedding = bert::embed(state.bert_model, state.bert_tokenizer, &payload.sentence)?;
    let response = EmbedResponse { embedding };
    Ok(Json(response))
}
