use std::{collections::HashMap, env, ops::DerefMut, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use tokio::{
    self,
    sync::{Mutex, Semaphore},
};

use crate::service::{self, hacker_news, inference, search_engine};
use crate::{command, repository::Repository};

pub(crate) async fn collect_items(repo: Arc<Mutex<Repository>>) -> Result<()> {
    let items_num: i32 = env::var("JOB_COLLECT_ITEMS_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let permits_num: usize = env::var("JOB_PERMITS_NUM").unwrap_or("100".to_string()).parse()?;
    let chunk_size: i32 = env::var("JOB_CHUNK_SIZE").unwrap_or("1000".to_string()).parse()?;
    let max_id = hacker_news::get_max_item_id().await?;
    let min_id = std::cmp::max(0, max_id - (items_num - 1));
    // Iterate in reverse order
    let mut chunk_max_id = max_id;
    while chunk_max_id >= min_id {
        let chunk_min_id = std::cmp::max(min_id, chunk_max_id - chunk_size + 1);
        collect_chunk_items(Arc::clone(&repo), permits_num, chunk_min_id, chunk_max_id).await?;
        chunk_max_id -= chunk_size;
    }
    Ok(())
}

pub(crate) async fn collect_item_urls(repo: Arc<Mutex<Repository>>) -> Result<()> {
    let items_num: i32 = env::var("JOB_COLLECT_ITEM_URLS_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let permits_num: usize = env::var("JOB_PERMITS_NUM").unwrap_or("10".to_string()).parse()?;
    let chunk_size: i32 = env::var("JOB_CHUNK_SIZE").unwrap_or("1000".to_string()).parse()?;
    let replicas_num: i32 = env::var("JOB_REPLICAS_NUM").unwrap_or("1".to_string()).parse()?;
    let replica_index: i32 = env::var("JOB_REPLICA_INDEX").unwrap_or("0".to_string()).parse()?;
    // TODO: Handle the case where there is no item record
    let max_id = repo.lock().await.deref_mut().find_max_item_id()?;
    let min_id = std::cmp::max(
        max_id - (items_num - 1),
        repo.lock().await.deref_mut().find_min_item_id()?,
    );
    // Iterate in reverse order
    let mut chunk_max_id = max_id;
    while chunk_max_id >= min_id {
        let chunk_min_id = std::cmp::max(min_id, chunk_max_id - chunk_size + 1);
        collect_chunk_item_urls(
            Arc::clone(&repo),
            permits_num,
            chunk_min_id,
            chunk_max_id,
            replicas_num,
            replica_index,
        )
        .await?;
        chunk_max_id -= chunk_size;
    }
    Ok(())
}

pub(crate) async fn summarize_texts(mut repo: Repository) -> Result<()> {
    let texts_num: usize = env::var("JOB_SUMMARIZE_TEXTS_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    let top_story_ids = hacker_news::get_top_story_ids().await?;
    let mut items = repo.find_summary_missing_items(&top_story_ids)?;
    // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
    //   as `LIMIT` doesn't maintain the order of top stories' ids.
    items.truncate(texts_num);
    if env::var("JOB_SUMMARIZE_ADDITIONAL_TEXTS").is_ok() && items.len() < texts_num {
        let mut additional_items =
            repo.find_summary_missing_items_excluding(&top_story_ids, texts_num - items.len())?;
        items.append(&mut additional_items);
    }
    for (id, title, text) in items {
        let shortened_text = command::shorten_text(&text)?;
        let start_time = std::time::Instant::now();
        let summary = match inference::instruct_summary(&title, &shortened_text).await {
            Ok(summary) => summary,
            Err(e) => {
                println!("[ERR] inference.instruct_summary (id={id}): err={e}");
                continue;
            }
        };
        println!(
            "[INFO] main.summarize_texts (id={}): shortened_text.len={}, summary.len={}, elapsed_time={:?}",
            id,
            shortened_text.len(),
            summary.len(),
            start_time.elapsed()
        );
        repo.update_item_url(id, summary)?;
    }
    Ok(())
}

pub(crate) async fn embed_summaries(mut repo: Repository) -> Result<()> {
    let collection_name = env::var("SEARCH_ENGINE_VECTOR_SUMMARY_COLLECTION_NAME")?;
    let summaries_num: usize = env::var("JOB_EMBED_SUMMARIES_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let chunk_size: usize = env::var("JOB_CHUNK_SIZE").unwrap_or("50".to_string()).parse()?;
    let summary_existing_ids = repo.find_summary_existing_items(summaries_num)?;
    let embedding_missing_ids = search_engine::find_missing(collection_name.clone(), summary_existing_ids).await?;
    for chunk in embedding_missing_ids.chunks(chunk_size) {
        let item_summaries = repo.find_item_summaries(chunk)?;
        for (id, text, summary) in item_summaries {
            let sentence = if let Some(text) = text {
                text
            } else if let Some(summary) = summary {
                summary
            } else {
                continue;
            };
            let embedding = inference::embed(sentence).await?;
            search_engine::upsert(collection_name.clone(), id, embedding).await?;
            println!("[INFO] main.embed_summaries (id={})", id);
        }
    }
    Ok(())
}

async fn collect_chunk_items(
    repo: Arc<Mutex<Repository>>,
    permits_num: usize,
    chunk_min_id: i32,
    chunk_max_id: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let item_ids = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_items(chunk_min_id, chunk_max_id)?;
    // Iterate in reverse order
    for id in item_ids.into_iter().rev() {
        let permit = semaphore.clone().acquire_owned().await?;
        let repo_inst = Arc::clone(&repo);
        let handle = tokio::spawn(async move {
            let max_retry_count = 100;
            let mut retry_count = 0;
            let item = loop {
                match hacker_news::get_item(id).await {
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
            Err(e) => println!("[ERR] main.collect_chunk_items.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}

async fn collect_chunk_item_urls(
    repo: Arc<Mutex<Repository>>,
    permits_num: usize,
    chunk_min_id: i32,
    chunk_max_id: i32,
    replicas_num: i32,
    replica_index: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let item_urls = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_item_urls(chunk_min_id, chunk_max_id)?;
    // Iterate in reverse order
    for (id, url) in item_urls.into_iter().rev() {
        if id % replicas_num != replica_index {
            continue;
        }
        let permit = semaphore.clone().acquire_owned().await?;
        let repo_inst = Arc::clone(&repo);
        let handle = tokio::spawn(tokio::time::timeout(Duration::from_secs(300), async move {
            let item_url = match hacker_news::get_item_url(&url).await {
                Ok(item_url) => item_url,
                Err(e) => service::ItemUrl::Canceled { note: e.to_string() },
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
            Err(e) => println!("[ERR] main.collect_chunk_item_urls.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}
