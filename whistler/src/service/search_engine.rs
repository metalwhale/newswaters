use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SearchSimilarRequest {
    embedding: Vec<f32>,
    limit: u64,
}

#[derive(Deserialize)]
struct SearchSimilarResponse {
    items: Vec<(i32, f32)>,
}

pub(crate) async fn search_similar(embedding: Vec<f32>, limit: u64) -> Result<Vec<(i32, f32)>> {
    let payload = SearchSimilarRequest { embedding, limit };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/search-similar",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .json::<SearchSimilarResponse>()
        .await?;
    let items = response.items;
    Ok(items)
}
