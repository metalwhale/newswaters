use std::{collections::HashMap, env};

use anyhow::{Context, Result};
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

enum LeafQuery {
    Lexical(String),
    Semantic(Vec<f32>),
}

// See:
// - https://opster.com/guides/opensearch/opensearch-machine-learning/opensearch-hybrid-search/
// - https://opensearch.org/blog/semantic-science-benchmarks/
pub(crate) async fn search_similar(sentence: String, embedding: Vec<f32>, limit: usize) -> Result<Vec<(i32, f32)>> {
    // Obtain the results from each individual leaf query
    let lexical_results = search_similar_leaf(
        LeafQuery::Lexical(sentence),
        env::var("WHISTLER_SEARCH_SIMILAR_LEXICAL_LIMIT")
            .unwrap_or((limit * 50).to_string())
            .parse()?,
    )
    .await?;
    let semantic_results = search_similar_leaf(
        LeafQuery::Semantic(embedding),
        env::var("WHISTLER_SEARCH_SIMILAR_SEMANTIC_LIMIT")
            .unwrap_or((limit * 50).to_string())
            .parse()?,
    )
    .await?;
    // Combining them
    let lexical_weight: f32 = env::var("WHISTLER_SEARCH_SIMILAR_LEXICAL_WEIGHT")
        .unwrap_or("0.25".to_string())
        .parse()?;
    let mut weighted_lexical_results = HashMap::new();
    if lexical_results.len() > 0 {
        let (_, lexical_max_score) = lexical_results.first().context("Lexical max score")?.to_owned();
        let (_, lexical_min_score) = lexical_results.last().context("Lexical min score")?.to_owned();
        for (id, score) in &lexical_results {
            let weighted_score = if lexical_results.len() > 1 {
                lexical_weight * (score - lexical_min_score) / (lexical_max_score - lexical_min_score)
            } else {
                lexical_weight
            };
            weighted_lexical_results.insert(id, weighted_score);
        }
    }
    let semantic_weight = 1.0 - lexical_weight;
    let mut compound_results = vec![];
    if semantic_results.len() > 0 {
        let (_, semantic_max_score) = semantic_results.first().context("Semantic max score")?.to_owned();
        let (_, semantic_min_score) = semantic_results.last().context("Semantic min score")?.to_owned();
        for (id, score) in &semantic_results {
            let weighted_semantic_score = if semantic_results.len() > 1 {
                semantic_weight * (score - semantic_min_score) / (semantic_max_score - semantic_min_score)
            } else {
                semantic_weight
            };
            let score = if let Some(weighted_lexical_score) = weighted_lexical_results.remove(&id) {
                weighted_lexical_score + weighted_semantic_score
            } else {
                weighted_semantic_score
            };
            compound_results.push((*id, score));
        }
    }
    for (id, score) in weighted_lexical_results {
        compound_results.push((*id, score));
    }
    compound_results.sort_by(|(_, score1), (_, score2)| {
        score1
            .partial_cmp(score2)
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
    });
    compound_results.truncate(limit);
    return Ok(compound_results);
}

async fn search_similar_leaf(query: LeafQuery, limit: u64) -> Result<Vec<(i32, f32)>> {
    let payload = match query {
        LeafQuery::Lexical(sentence) => SearchSimilarRequest {
            sentence: Some(sentence.replace("\"", "")),
            embedding: None,
            limit,
        },
        LeafQuery::Semantic(embedding) => SearchSimilarRequest {
            sentence: None,
            embedding: Some(embedding),
            limit,
        },
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
