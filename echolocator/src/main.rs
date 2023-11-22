mod bert;
mod llama;

use std::env;

use anyhow::{Error, Result};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

use crate::bert::Bert;
use crate::llama::Llama;

#[derive(Clone)]
struct AppState {
    llama: Llama,
    bert: Bert,
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
    let state = initialize().await?;
    println!("[INFO] main.initialize");
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

async fn initialize() -> Result<AppState> {
    let llama = Llama::new().await?;
    let bert = Bert::new()?;
    let state = AppState { llama, bert };
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
    let completion = state.llama.inference(&payload.instruction).await?;
    let response = InstructResponse { completion };
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
    State(mut state): State<AppState>,
    Json(payload): Json<EmbedRequest>,
) -> Result<Json<EmbedResponse>, AppError> {
    let embedding = state.bert.embed(&payload.sentence)?;
    let response = EmbedResponse { embedding };
    Ok(Json(response))
}
