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
            // Item
            "collect_items" => command::item::collect_items(Arc::new(Mutex::new(repo))).await?,
            "collect_item_urls" => command::item::collect_item_urls(Arc::new(Mutex::new(repo))).await?,
            "summarize_texts" => command::item::summarize_texts(repo).await?,
            "embed_summaries" => command::item::embed_summaries(repo).await?,
            // Analysis
            "analyze_texts" => command::analysis::analyze_texts(repo).await?,
            "embed_keywords" => command::analysis::embed_keywords(repo).await?,
            "analyze_summaries" => command::analysis::analyze_summaries(repo).await?,
            _ => {}
        }
    }
    Ok(())
}