use std::{collections::HashMap, env, time::Duration};

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct InstructResponse {
    completion: String,
}

pub(crate) async fn instruct_summary(title: &str, text: &str) -> Result<String> {
    let instruction = format!(
        "\
        Please generate related topics and provide a detailed summary that aligns with the title and omits any irrelevant text. \
        Output only the title if the content is not related to it. \
        Don't make up information if it's not provided.\n\n\
        Title:\n\
        {}\n\n\
        Content:\n\
        {}\n\n\
        Output format:\n\
        - Topics:\n\
        - Summary:\n\
        ",
        title, text
    );
    let mut payload = HashMap::new();
    payload.insert("instruction", instruction);
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/instruct",
        env::var("ECHOLOCATOR_HOST")?,
        env::var("ECHOLOCATOR_PORT")?
    );
    let response = client
        .post(endpoint)
        .timeout(Duration::from_secs(600))
        .json(&payload)
        .send()
        .await?
        .json::<InstructResponse>()
        .await?;
    let summary = response.completion;
    Ok(summary)
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

pub(crate) async fn embed(sentence: &str) -> Result<Vec<f32>> {
    let mut payload = HashMap::new();
    // See: https://github.com/FlagOpen/FlagEmbedding/tree/b755dff/FlagEmbedding/llm_embedder#using-transformers
    payload.insert("sentence", format!("Represent this document for retrieval: {sentence}"));
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
