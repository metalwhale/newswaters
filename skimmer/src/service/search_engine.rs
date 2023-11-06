use std::{collections::HashMap, env};

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct FindMissingResponse {
    missing_ids: Vec<i32>,
}

pub(crate) async fn find_missing(ids: Vec<i32>) -> Result<Vec<i32>> {
    let mut payload = HashMap::new();
    payload.insert("ids", ids);
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/find-missing",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .json::<FindMissingResponse>()
        .await?;
    let missing_ids = response.missing_ids;
    Ok(missing_ids)
}

#[derive(Serialize)]
struct UpsertRequest {
    id: i32,
    sentence: String,
    embedding: Vec<f32>,
}

pub(crate) async fn upsert(id: i32, sentence: String, embedding: Vec<f32>) -> Result<()> {
    let payload = UpsertRequest {
        id,
        sentence,
        embedding,
    };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/upsert",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    client.post(endpoint).json(&payload).send().await?;
    Ok(())
}
