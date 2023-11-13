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
    let is_job = env::var("SKIMMER_IS_JOB").is_ok();
    if args.len() >= 2 {
        match args[1].as_str() {
            // Item
            "collect_items" => command::item::collect_items(Arc::new(Mutex::new(repo)), is_job).await?,
            "collect_item_urls" => command::item::collect_item_urls(Arc::new(Mutex::new(repo)), is_job).await?,
            "summarize_texts" => command::item::summarize_texts(repo, is_job).await?,
            "embed_summaries" => command::item::embed_summaries(repo, is_job).await?,
            // Analysis
            "analyze_texts" => command::analysis::analyze_texts(repo, is_job).await?,
            "embed_keywords" => command::analysis::embed_keywords(repo, is_job).await?,
            _ => {}
        }
    }
    Ok(())
}
