use std::env;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct FindMissingRequest {
    ids: Vec<i32>,
}

#[derive(Deserialize)]
struct FindMissingResponse {
    missing_ids: Vec<i32>,
}

pub(crate) async fn find_missing(ids: Vec<i32>) -> Result<Vec<i32>> {
    let payload = FindMissingRequest { ids };
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
    embedding: Vec<f32>,
}

pub(crate) async fn upsert(id: i32, embedding: Vec<f32>) -> Result<()> {
    let payload = UpsertRequest { id, embedding };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/upsert",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    client.post(endpoint).json(&payload).send().await?;
    Ok(())
}
