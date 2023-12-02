mod command;
mod repository;
mod schema;
mod service;

use std::{env, sync::Arc};

use anyhow::Result;
use tokio::{self, sync::Mutex};

use crate::repository::Repository;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let repo = Repository::new()?;
    if args.len() >= 2 {
        match args[1].as_str() {
            // Collecting
            "collect-items" => command::item::collect_items(Arc::new(Mutex::new(repo))).await?,
            "collect-item-urls" => command::item::collect_item_urls(Arc::new(Mutex::new(repo))).await?,
            // Summarize and analyze
            "summarize-texts" => command::item::summarize_texts(repo).await?,
            "analyze-story-texts" => command::analysis::analyze_story_texts(repo).await?,
            "analyze-comment-texts" => command::analysis::analyze_comment_texts(repo).await?,
            "analyze-summaries" => command::analysis::analyze_summaries(repo).await?,
            // Embedding
            "embed-summaries" => command::item::embed_summaries(repo).await?,
            "embed-keywords" => command::analysis::embed_keywords(repo).await?,
            _ => {}
        }
    }
    Ok(())
}
