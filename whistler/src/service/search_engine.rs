use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SearchRequest {
    embedding: Vec<f32>,
    limit: u64,
}

#[derive(Deserialize)]
struct SearchResponse {
    points: Vec<(i32, f32)>,
}

pub(crate) async fn search(embedding: Vec<f32>, limit: u64) -> Result<Vec<(i32, f32)>> {
    let payload = SearchRequest { embedding, limit };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/search",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .json::<SearchResponse>()
        .await?;
    let points = response.points;
    Ok(points)
}
