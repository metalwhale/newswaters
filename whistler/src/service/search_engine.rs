use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SearchSimilarRequest {
    sentence: Option<String>,
    embedding: Option<Vec<f32>>,
    limit: u64,
}

#[derive(Deserialize)]
struct SearchSimilarResponse {
    items: Vec<(i32, f32)>,
}

pub(crate) async fn search_similar(sentence: String, embedding: Vec<f32>, limit: u64) -> Result<Vec<(i32, f32)>> {
    let payload = if sentence.starts_with("\"") && sentence.ends_with("\"") {
        SearchSimilarRequest {
            sentence: Some(sentence.replace("\"", "")),
            embedding: None,
            limit,
        }
    } else {
        SearchSimilarRequest {
            sentence: None,
            embedding: Some(embedding),
            limit,
        }
    };
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
