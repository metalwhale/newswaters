use std::{collections::HashMap, env};

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

// TODO: DRY this function to eliminate duplication with a similar one in "skimmer".
pub(crate) async fn embed(sentence: &str) -> Result<Vec<f32>> {
    let mut payload = HashMap::new();
    // See: https://github.com/FlagOpen/FlagEmbedding/tree/b755dff/FlagEmbedding/llm_embedder#using-transformers
    payload.insert(
        "sentence",
        format!("Represent this query for retrieving relevant documents: {sentence}"),
    );
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/embed",
        env::var("ECHOLOCATOR_HOST")?,
        env::var("ECHOLOCATOR_PORT")?
    );
    let response = client
        .post(endpoint)
        .json(&payload)
        .send()
        .await?
        .json::<EmbedResponse>()
        .await?;
    let embedding = response.embedding;
    Ok(embedding)
}
