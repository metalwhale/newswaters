use std::{env, process::Command};

use anyhow::{Context, Error, Result};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use futures_util::StreamExt;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::{fs::File, io::AsyncWriteExt};

#[derive(Clone)]
struct AppState {
    model_path: String,
    prompt_template: String,
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
    println!("[INFO] main: state.model_path={}", &state.model_path);
    let app = Router::new()
        .route("/healthz", routing::get(|| async { "Ok" }))
        .route("/instruct", routing::post(instruct))
        .with_state(state);
    let port = env::var("ECHOLOCATOR_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn initialize(model_dir: &str) -> Result<AppState> {
    let model_url = env::var("ECHOLOCATOR_MODEL_URL").unwrap_or(
        "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.Q5_K_M.gguf".to_string(),
    );
    let prompt_template = env::var("ECHOLOCATOR_PROMPT_TEMPLATE").unwrap_or("<s>[INST] {prompt} [/INST]".to_string());
    let model_name = model_url
        .split("/")
        .last()
        .context(format!("model_url={model_url}"))?
        .to_string();
    let model_path = format!("{model_dir}/{model_name}");
    let state = AppState {
        model_path,
        prompt_template,
    };
    if std::path::Path::new(&state.model_path).exists() {
        return Ok(state);
    }
    let mut file = File::create(&state.model_path).await?;
    let mut stream = reqwest::get(model_url).await?.bytes_stream();
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
    let prompt = state.prompt_template.replace("{prompt}", &payload.instruction);
    let output = Command::new("llama")
        .args([
            "--model",
            &state.model_path,
            "--ctx-size",
            &env::var("ECHOLOCATOR_CTX_SIZE").unwrap_or("8192".to_string()),
            "--temp",
            &env::var("ECHOLOCATOR_TEMP").unwrap_or("0.8".to_string()),
            "--repeat-penalty",
            &env::var("ECHOLOCATOR_REPEAT_PENALTY").unwrap_or("1.2".to_string()),
            "--prompt",
            &prompt,
            "--log-disable",
        ])
        .output()?;
    let response = InstructResponse {
        completion: String::from_utf8(output.stdout)?
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
