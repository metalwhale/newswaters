mod repository;
mod service;

use std::{env, sync::Arc};

use anyhow::{Error, Result};
use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing, Json, Router,
};
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use service::search_engine;
use tower_http::cors::{Any, CorsLayer};

use crate::repository::Repository;
use crate::service::inference;

#[derive(Clone)]
struct AppState {
    repo: Arc<Repository>,
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
    let prefix = env::var("WHISTLER_PREFIX").unwrap_or("".to_string());
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([reqwest::header::CONTENT_TYPE])
        .allow_origin(Any);
    let app = Router::new()
        .nest(
            &prefix,
            Router::new()
                .route("/healthz", routing::get(|| async { "Ok" }))
                .route("/search-similar-items", routing::post(search_similar_items)),
        )
        .layer(cors)
        .with_state(state);
    let port = env::var("WHISTLER_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn initialize() -> Result<AppState> {
    let repo = Arc::new(Repository::new()?);
    let state = AppState { repo };
    return Ok(state);
}

#[derive(Deserialize)]
struct SearchSimilarItemsRequest {
    sentence: String,
    limit: u64,
}

#[derive(Serialize)]
struct SearchSimilarItemsResponse {
    items: Vec<(i32, f32, Option<String>, Option<String>)>,
}

async fn search_similar_items(
    State(state): State<AppState>,
    Json(payload): Json<SearchSimilarItemsRequest>,
) -> Result<Json<SearchSimilarItemsResponse>, AppError> {
    let embedding = inference::embed(&payload.sentence).await?;
    let points = search_engine::search(embedding, payload.limit).await?;
    let ids = points.iter().map(|(id, _score)| *id).collect::<Vec<i32>>();
    let mut items_map = state.repo.find_items(&ids)?;
    let mut items = vec![];
    for (id, score) in points {
        if let Some((title, url)) = items_map.remove(&id) {
            items.push((id, score, title, url))
        }
    }
    let response = SearchSimilarItemsResponse { items };
    Ok(Json(response))
}
