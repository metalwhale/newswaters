use std::{env, process::Command};

use anyhow::{Error, Result};
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
    let model_path = "/usr/src/models/mistral-7b-instruct-v0.1.Q4_K_M.gguf".to_string();
    download_model_file(&model_path).await?;
    println!("[INFO] main: Model downloaded");
    let state = AppState { model_path };
    let app = Router::new()
        .route("/healthz", routing::get(|| async { "Ok" }))
        .route("/inference", routing::post(inference))
        .with_state(state);
    let port = env::var("ECHOLOCATOR_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn download_model_file(model_path: &str) -> Result<()> {
    if std::path::Path::new(model_path).exists() {
        return Ok(());
    }
    let model_url = "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.Q4_K_M.gguf";
    let mut file = File::create(model_path).await?;
    let mut stream = reqwest::get(model_url).await?.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(())
}

#[derive(Deserialize)]
struct InferenceRequest {
    prompt: String,
}

#[derive(Serialize)]
struct InferenceResponse {
    completion: String,
}

async fn inference(
    State(state): State<AppState>,
    Json(payload): Json<InferenceRequest>,
) -> Result<Json<InferenceResponse>, AppError> {
    let output = Command::new("llama")
        .args([
            "--model",
            &state.model_path,
            "--ctx-size",
            "8192",
            "--temp",
            "0.0",
            "--prompt",
            &payload.prompt,
            "--log-disable",
        ])
        .output()?;
    let response = InferenceResponse {
        completion: String::from_utf8(output.stdout)?,
    };
    Ok(Json(response))
}
