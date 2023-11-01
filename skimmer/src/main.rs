mod repository;
mod schema;
mod service;
mod vector_repository;

use std::{collections::HashMap, env, ops::DerefMut, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use tokio::{
    self,
    sync::{Mutex, Semaphore},
};

use crate::repository::Repository;
use crate::service::ItemUrl;
use crate::vector_repository::VectorRepository;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let repo = Repository::new()?;
    let is_job = match env::var("SKIMMER_IS_JOB") {
        Ok(_) => true,
        Err(_) => false,
    };
    if args.len() >= 2 {
        match args[1].as_str() {
            "collect_items" => collect_items(Arc::new(Mutex::new(repo)), is_job).await?,
            "collect_item_urls" => collect_item_urls(Arc::new(Mutex::new(repo)), is_job).await?,
            "consume_top_stories" => consume_top_stories(repo, is_job).await?,
            "consume_top_story_summaries" => {
                let vector_repo = VectorRepository::new().await?;
                consume_top_story_summaries(repo, vector_repo, is_job).await?
            }
            _ => {}
        }
    }
    Ok(())
}

async fn collect_items(repo: Arc<Mutex<Repository>>, is_job: bool) -> Result<()> {
    let items_num: i32 = env::var("SKIMMER_ITEMS_NUM").unwrap_or("1000000".to_string()).parse()?;
    let permits_num: usize = env::var("SKIMMER_PERMITS_NUM").unwrap_or("100".to_string()).parse()?;
    let batch_size: i32 = env::var("SKIMMER_BATCH_SIZE").unwrap_or("1000".to_string()).parse()?;
    loop {
        let max_id = service::get_max_item_id().await?;
        let min_id = std::cmp::max(0, max_id - (items_num - 1));
        // Iterate in reverse order
        let mut batch_max_id = max_id;
        while batch_max_id >= min_id {
            let batch_min_id = std::cmp::max(min_id, batch_max_id - batch_size + 1);
            collect_batch_items(Arc::clone(&repo), permits_num, batch_min_id, batch_max_id).await?;
            batch_max_id -= batch_size;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

async fn collect_item_urls(repo: Arc<Mutex<Repository>>, is_job: bool) -> Result<()> {
    let items_num: i32 = env::var("SKIMMER_ITEM_URLS_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let permits_num: usize = env::var("SKIMMER_PERMITS_NUM").unwrap_or("10".to_string()).parse()?;
    let batch_size: i32 = env::var("SKIMMER_BATCH_SIZE").unwrap_or("1000".to_string()).parse()?;
    let replicas_num: i32 = env::var("SKIMMER_REPLICAS_NUM").unwrap_or("1".to_string()).parse()?;
    let replica_index: i32 = env::var("SKIMMER_REPLICA_INDEX").unwrap_or("0".to_string()).parse()?;
    loop {
        // TODO: Handle the case where there is no item record
        let max_id = repo.lock().await.deref_mut().find_max_item_id()?;
        let min_id = std::cmp::max(
            max_id - (items_num - 1),
            repo.lock().await.deref_mut().find_min_item_id()?,
        );
        // Iterate in reverse order
        let mut batch_max_id = max_id;
        while batch_max_id >= min_id {
            let batch_min_id = std::cmp::max(min_id, batch_max_id - batch_size + 1);
            collect_batch_item_urls(
                Arc::clone(&repo),
                permits_num,
                batch_min_id,
                batch_max_id,
                replicas_num,
                replica_index,
            )
            .await?;
            batch_max_id -= batch_size;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

async fn consume_top_stories(mut repo: Repository, is_job: bool) -> Result<()> {
    let top_stories_num: usize = env::var("SKIMMER_TOP_STORIES_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    let text_min_line_length: usize = env::var("SKIMMER_TEXT_MIN_LINE_LENGTH")
        .unwrap_or("80".to_string())
        .parse()?;
    let text_max_total_length: usize = env::var("SKIMMER_TEXT_MAX_TOTAL_LENGTH")
        .unwrap_or("2400".to_string())
        .parse()?;
    loop {
        let top_story_ids = service::get_top_story_ids().await?;
        let mut item_urls = repo.find_summary_missing_item_urls(&top_story_ids)?;
        // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
        //   as `LIMIT` doesn't maintain the order of top stories' ids.
        item_urls.truncate(top_stories_num);
        for (id, title, text) in item_urls {
            let shortened_text = shorten_text(&text, text_min_line_length, text_max_total_length);
            let start_time = std::time::Instant::now();
            let summary = match service::post_instruct_summary(&title, &shortened_text).await {
                Ok(summary) => summary,
                Err(e) => {
                    println!("[ERR] service.post_instruct_summary (id={id}): err={e}");
                    continue;
                }
            };
            println!(
                "[INFO] main.consume_top_stories (id={}): shortened_text.len={}, summary.len={}, elapsed_time={:?}",
                id,
                shortened_text.len(),
                summary.len(),
                start_time.elapsed()
            );
            repo.update_item_url(id, summary)?;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

async fn consume_top_story_summaries(mut repo: Repository, vector_repo: VectorRepository, is_job: bool) -> Result<()> {
    let top_story_summaries_num: usize = env::var("SKIMMER_TOP_STORY_SUMMARIES_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    loop {
        let top_story_ids = service::get_top_story_ids().await?;
        let missing_ids = vector_repo.find_missing_ids(&top_story_ids).await?;
        let mut item_summaries = repo.find_item_summaries(&missing_ids)?;
        item_summaries.truncate(top_story_summaries_num);
        let mut embeddings = vec![];
        for (id, text, summary) in item_summaries {
            let sentence = if let Some(text) = text {
                text
            } else if let Some(summary) = summary {
                summary
            } else {
                continue;
            };
            embeddings.push((id, service::post_embed(&sentence).await?));
            println!("[INFO] main.consume_top_story_summaries.post_embed (id={})", id);
        }
        if embeddings.len() > 0 {
            vector_repo.upsert_embeddings(embeddings).await?;
            println!("[INFO] main.consume_top_story_summaries.upsert_embeddings");
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

async fn collect_batch_items(
    repo: Arc<Mutex<Repository>>,
    permits_num: usize,
    batch_min_id: i32,
    batch_max_id: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let item_ids = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_items(batch_min_id, batch_max_id)?;
    // Iterate in reverse order
    for id in item_ids.into_iter().rev() {
        let permit = semaphore.clone().acquire_owned().await?;
        let repo_inst = Arc::clone(&repo);
        let handle = tokio::spawn(async move {
            let max_retry_count = 100;
            let mut retry_count = 0;
            let item = loop {
                match service::get_item(id).await {
                    Ok(item) => break item,
                    Err(e) => {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        retry_count += 1;
                        if retry_count >= max_retry_count {
                            bail!(e)
                        }
                        continue;
                    }
                }
            };
            match repo_inst.lock().await.deref_mut().insert_item(item) {
                Ok(_) => {}
                Err(e) => println!("[ERR] repo.insert_item (id={id}): err={e}"),
            };
            drop(permit);
            Ok(())
        });
        handles.insert(id, handle);
    }
    for (id, handle) in handles {
        match handle.await? {
            Ok(_) => {}
            Err(e) => println!("[ERR] main.collect_batch_items.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}

async fn collect_batch_item_urls(
    repo: Arc<Mutex<Repository>>,
    permits_num: usize,
    batch_min_id: i32,
    batch_max_id: i32,
    replicas_num: i32,
    replica_index: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let item_urls = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_item_urls(batch_min_id, batch_max_id)?;
    // Iterate in reverse order
    for (id, url) in item_urls.into_iter().rev() {
        if id % replicas_num != replica_index {
            continue;
        }
        let permit = semaphore.clone().acquire_owned().await?;
        let repo_inst = Arc::clone(&repo);
        let handle = tokio::spawn(tokio::time::timeout(Duration::from_secs(300), async move {
            let item_url = match service::get_item_url(&url).await {
                Ok(item_url) => item_url,
                Err(e) => ItemUrl::Canceled { note: e.to_string() },
            };
            match repo_inst.lock().await.deref_mut().insert_item_url(id, item_url) {
                Ok(_) => {}
                Err(e) => println!("[ERR] repo.insert_item_url (id={id}): err={e}"),
            };
            drop(permit);
        }));
        handles.insert(id, handle);
    }
    for (id, handle) in handles {
        match handle.await? {
            Ok(_) => {}
            Err(e) => println!("[ERR] main.collect_batch_item_urls.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}

fn shorten_text(text: &str, min_line_length: usize, max_total_length: usize) -> String {
    let mut lines = vec![];
    let mut total_length = 0;
    for line in text
        .split("\n")
        .into_iter()
        .map(|l| format!("- {}", l.trim()))
        .collect::<Vec<String>>()
    {
        let length = line.len();
        if total_length + length > max_total_length {
            continue;
        }
        if min_line_length <= length {
            lines.push(line);
            total_length += length;
        }
    }
    return lines.join("\n");
}
