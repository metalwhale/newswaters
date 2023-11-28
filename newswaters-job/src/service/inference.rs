use std::{env, time::Duration};

use anyhow::Result;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct InstructRequest {
    instruction: String,
}

#[derive(Deserialize)]
struct InstructResponse {
    completion: String,
}

pub(crate) async fn instruct_summary(title: &str, text: &str) -> Result<String> {
    let instruction = format!(
        "\
        Please generate related topics and provide a detailed summary that aligns with the title and omits any irrelevant text. \
        Don't output the title. \
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
    let summary = instruct(instruction).await?;
    return Ok(summary);
}

pub(crate) async fn instruct_keyword(title: &str, text: &str) -> Result<String> {
    let instruction = format!(
        "\
        Please generate related keywords that align with the title and omits any irrelevant text. \
        Output only the keywords without any additional explanation. \
        The keywords should be separated by commas. \
        Don't make up information if it's not provided.\n\n\
        Title:\n\
        {}\n\n\
        Content:\n\
        {}\n\n\
        ",
        title, text
    );
    let summary = instruct(instruction).await?;
    return Ok(summary);
}

pub(crate) async fn instruct_summary_anchor_query(summary: &str) -> Result<String> {
    let instruction = format!(
        "\
        Please generate a sentence aligning with the provided content, omitting irrelevant text. \
        Output the sentence without additional explanation. \
        Ensure it is fewer than {} words.\n\n\
        Content:\n\
        {}\n\n\
        ",
        env::var("JOB_INSTRUCT_SUMMARY_ANCHOR_QUERY_MAX_WORDS_COUNT").unwrap_or("20".to_string()),
        summary
    );
    let query = instruct(instruction).await?;
    return Ok(query);
}

pub(crate) async fn instruct_entailment_query(premise: &str) -> Result<String> {
    let instruction = format!(
        "Refine the following sentence while keeping its meaning unchanged. \
        Output the sentence without additional explanation.\n\n\
        \"{}\"\n\
        ",
        premise
    );
    let hypothesis = instruct(instruction).await?;
    return Ok(hypothesis);
}

pub(crate) async fn instruct_contradiction_query(premise: &str) -> Result<String> {
    let instruction = format!(
        "Make modifications to the following sentence, ensuring that its meaning becomes entirely contradictory. \
        Output the sentence without additional explanation.\n\n\
        \"{}\"\n\
        ",
        premise
    );
    let hypothesis = instruct(instruction).await?;
    return Ok(hypothesis);
}

pub(crate) async fn instruct_random_query(original: &str) -> Result<String> {
    let mut words = original
        .split(" ")
        .map(|n| n.to_string().to_lowercase())
        .collect::<Vec<String>>();
    let sentence_len = words.len();
    words.shuffle(&mut rand::thread_rng());
    words.truncate(
        (sentence_len as f32
            * env::var("JOB_INSTRUCT_RANDOM_QUERY_WORDS_RETENTION_RATE")
                .unwrap_or("0.1".to_string())
                .parse::<f32>()?) as usize
            + 1,
    );
    let instruction = format!(
        "Generate a random sentence using the provided words. \
        Ensure the sentence contains a minimum of {} words. \
        Output the sentence without additional explanation.\n\n\
        \"{}\"\n\
        ",
        sentence_len,
        words.join(", ")
    );
    let hypothesis = instruct(instruction).await?;
    return Ok(hypothesis);
}

pub(crate) async fn instruct_subject_query(content: &str) -> Result<String> {
    let instruction = format!(
        "\
        Please generate {} different subjects aligning with the content. \
        Output subjects without additional explanation. \
        Output each subject on a separate line. \
        Each subject must consist of fewer than {} words.\n\n\
        Content:\n\
        {}\n\n\
        ",
        env::var("JOB_INSTRUCT_SUBJECT_QUERY_MAX_SUBJECTS_NUM").unwrap_or("5".to_string()),
        env::var("JOB_INSTRUCT_SUBJECT_QUERY_MAX_WORDS_COUNT").unwrap_or("5".to_string()),
        content
    );
    let subject = instruct(instruction).await?;
    return Ok(subject);
}

async fn instruct(instruction: String) -> Result<String> {
    let payload = InstructRequest { instruction };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/instruct",
        env::var("INFERENCE_HOST")?,
        env::var("INFERENCE_PORT")?
    );
    let response = client
        .post(endpoint)
        .timeout(Duration::from_secs(600))
        .json(&payload)
        .send()
        .await?
        .json::<InstructResponse>()
        .await?;
    return Ok(response.completion);
}

#[derive(Serialize)]
struct EmbedRequest {
    sentence: String,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embedding: Vec<f32>,
}

pub(crate) async fn embed(sentence: String) -> Result<Vec<f32>> {
    let payload = EmbedRequest { sentence };
    let client = reqwest::Client::new();
    let endpoint = format!(
        "http://{}:{}/embed",
        env::var("INFERENCE_HOST")?,
        env::var("INFERENCE_PORT")?
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
