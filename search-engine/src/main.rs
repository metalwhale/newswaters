#[macro_use]
extern crate tantivy;

mod text_repository;
mod vector_repository;

use std::{env, sync::Arc};

use anyhow::{Error, Result};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use serde::{Deserialize, Serialize};
use text_repository::TextRepository;

use crate::vector_repository::VectorRepository;

#[derive(Clone)]
struct AppState {
    vector_repo: Arc<VectorRepository>,
    text_repo: TextRepository,
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
    let app = Router::new()
        .route("/healthz", routing::get(|| async { "Ok" }))
        .route("/find-missing", routing::post(find_missing))
        .route("/upsert", routing::post(upsert))
        .route("/search-similar", routing::post(search_similar))
        .with_state(state);
    let port = env::var("SEARCH_ENGINE_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn initialize() -> Result<AppState> {
    let vector_repo = Arc::new(VectorRepository::new().await?);
    let text_repo = TextRepository::new()?;
    let state = AppState { vector_repo, text_repo };
    return Ok(state);
}

#[derive(Deserialize)]
struct FindMissingRequest {
    ids: Vec<i32>,
}

#[derive(Serialize)]
struct FindMissingResponse {
    missing_ids: Vec<i32>,
}

async fn find_missing(
    State(state): State<AppState>,
    Json(payload): Json<FindMissingRequest>,
) -> Result<Json<FindMissingResponse>, AppError> {
    // TODO: Find missing ids in the text repo on its own, rather than relying on the vector repo
    let missing_ids = state.vector_repo.find_missing(payload.ids).await?;
    let response = FindMissingResponse { missing_ids };
    Ok(Json(response))
}

#[derive(Deserialize)]
struct UpsertRequest {
    id: i32,
    sentence: String,
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct UpsertResponse {}

async fn upsert(
    State(state): State<AppState>,
    Json(payload): Json<UpsertRequest>,
) -> Result<Json<UpsertResponse>, AppError> {
    state.vector_repo.upsert(payload.id, payload.embedding).await?;
    state.text_repo.add(payload.id, payload.sentence)?;
    let response = UpsertResponse {};
    Ok(Json(response))
}

#[derive(Deserialize)]
struct SearchSimilarRequest {
    sentence: Option<String>,
    embedding: Option<Vec<f32>>,
    limit: u64,
}

#[derive(Serialize)]
struct SearchSimilarResponse {
    items: Vec<(i32, f32)>,
}

async fn search_similar(
    State(state): State<AppState>,
    Json(payload): Json<SearchSimilarRequest>,
) -> Result<Json<SearchSimilarResponse>, AppError> {
    let items = if let Some(sentence) = payload.sentence {
        state.text_repo.search_similar(sentence, payload.limit as usize)?
    } else if let Some(embedding) = payload.embedding {
        state.vector_repo.search_similar(embedding, payload.limit).await?
    } else {
        vec![]
    };
    let response = SearchSimilarResponse { items };
    Ok(Json(response))
}
