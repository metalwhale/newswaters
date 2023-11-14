use std::{collections::HashMap, env};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct SearchSimilarRequest {
    collection_name: String,
    embedding: Vec<f32>,
    limit: u64,
}

#[derive(Deserialize)]
struct SearchSimilarResponse {
    items: Vec<(i32, f32)>,
}

pub(crate) async fn search_similar(embedding: Vec<f32>, limit: u64) -> Result<Vec<(i32, f32)>> {
    let collection_names = env::var("SEARCH_ENGINE_VECTOR_COLLECTION_NAMES")?
        .split(",")
        .map(|n| n.to_string())
        .collect::<Vec<String>>();
    let mut compound_items = HashMap::new();
    for collection_name in &collection_names {
        let payload = SearchSimilarRequest {
            collection_name: collection_name.clone(),
            embedding: embedding.clone(),
            limit,
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
        if collection_names.len() == 1 {
            return Ok(items);
        } else {
            for (id, score) in items {
                let weighted_score = score / collection_names.len() as f32;
                compound_items
                    .entry(id)
                    .and_modify(|score| *score += weighted_score)
                    .or_insert(weighted_score);
            }
        }
    }
    let mut items = compound_items
        .into_iter()
        .map(|(id, score)| (id, score))
        .collect::<Vec<(i32, f32)>>();
    items.sort_by(|(_, score1), (_, score2)| {
        score1
            .partial_cmp(score2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
    });
    items.truncate(limit as usize);
    Ok(items)
}
