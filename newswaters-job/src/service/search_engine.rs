use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct FindMissingRequest {
    collection_name: String,
    ids: Vec<i32>,
}

#[derive(Deserialize)]
struct FindMissingResponse {
    missing_ids: Vec<i32>,
}

pub(crate) async fn find_missing(collection_name: String, ids: Vec<i32>) -> Result<Vec<i32>> {
    let payload = FindMissingRequest { collection_name, ids };
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
    collection_name: String,
    id: i32,
    embedding: Vec<f32>,
}

pub(crate) async fn upsert(collection_name: String, id: i32, embedding: Vec<f32>) -> Result<()> {
    let payload = UpsertRequest {
        collection_name,
        id,
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
