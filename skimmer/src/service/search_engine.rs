use std::{collections::HashMap, env};

use anyhow::Result;
use serde::Deserialize;

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

#[derive(Deserialize)]
struct UpsertResponse {}

pub(crate) async fn upsert(embeddings: Vec<(i32, Vec<f32>)>) -> Result<()> {
    let mut payload = HashMap::new();
    payload.insert("embeddings", embeddings);
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/upsert",
        env::var("SEARCH_ENGINE_HOST")?,
        env::var("SEARCH_ENGINE_PORT")?
    );
    client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .json::<UpsertResponse>()
        .await?;
    Ok(())
}
