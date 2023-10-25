mod repository;
mod schema;
mod service;

use std::{collections::HashMap, env, ops::DerefMut, sync::Arc, time::Duration};

use anyhow::{bail, Result};
use tokio::{
    self,
    sync::{Mutex, Semaphore},
};

use crate::repository::ItemRepository;
use crate::service::ItemUrl;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let repo = Arc::new(Mutex::new(ItemRepository::new()?));
    let is_job = match env::var("SKIMMER_IS_JOB") {
        Ok(_) => true,
        Err(_) => false,
    };
    if args.len() >= 2 {
        match args[1].as_str() {
            "items" => save_items(repo, is_job).await?,
            "item_urls" => save_item_urls(repo, is_job).await?,
            _ => {}
        }
    }
    Ok(())
}

async fn save_items(repo: Arc<Mutex<ItemRepository>>, is_job: bool) -> Result<()> {
    let items_num: i32 = env::var("SKIMMER_ITEMS_NUM").unwrap_or("1000000".to_string()).parse()?;
    let permits_num: usize = env::var("SKIMMER_PERMITS_NUM").unwrap_or("100".to_string()).parse()?;
    let batch_size: i32 = env::var("SKIMMER_BATCH_SIZE").unwrap_or("1000".to_string()).parse()?;
    loop {
        let max_id = service::get_max_item().await?;
        let min_id = std::cmp::max(0, max_id - (items_num - 1));
        // Iterate in reverse order
        let mut batch_max_id = max_id;
        while batch_max_id >= min_id {
            let batch_min_id = std::cmp::max(min_id, batch_max_id - batch_size + 1);
            save_batch_items(Arc::clone(&repo), permits_num, batch_min_id, batch_max_id).await?;
            batch_max_id -= batch_size;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

async fn save_item_urls(repo: Arc<Mutex<ItemRepository>>, is_job: bool) -> Result<()> {
    let items_num: i32 = env::var("SKIMMER_ITEMS_NUM").unwrap_or("1000000".to_string()).parse()?;
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
            save_batch_item_urls(
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

async fn save_batch_items(
    repo: Arc<Mutex<ItemRepository>>,
    permits_num: usize,
    batch_min_id: i32,
    batch_max_id: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let ids = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_items(batch_min_id, batch_max_id)?;
    // Iterate in reverse order
    for id in ids.into_iter().rev() {
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
            match repo_inst.lock().await.deref_mut().save_item(item) {
                Ok(_) => {}
                Err(e) => println!("[ERR] repo.save_item (id={id}): err={e}"),
            };
            drop(permit);
            Ok(())
        });
        handles.insert(id, handle);
    }
    for (id, handle) in handles {
        match handle.await? {
            Ok(_) => {}
            Err(e) => println!("[ERR] main.save_batch_items.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}

async fn save_batch_item_urls(
    repo: Arc<Mutex<ItemRepository>>,
    permits_num: usize,
    batch_min_id: i32,
    batch_max_id: i32,
    replicas_num: i32,
    replica_index: i32,
) -> Result<()> {
    let semaphore = Arc::new(Semaphore::new(permits_num));
    let mut handles = HashMap::new();
    let ids = Arc::clone(&repo)
        .lock()
        .await
        .deref_mut()
        .find_missing_item_urls(batch_min_id, batch_max_id)?;
    // Iterate in reverse order
    for (id, url) in ids.into_iter().rev() {
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
            match repo_inst.lock().await.deref_mut().save_item_url(id, item_url) {
                Ok(_) => {}
                Err(e) => println!("[ERR] repo.save_item_url (id={id}): err={e}"),
            };
            drop(permit);
        }));
        handles.insert(id, handle);
    }
    for (id, handle) in handles {
        match handle.await? {
            Ok(_) => {}
            Err(e) => println!("[ERR] main.save_batch_item_urls.handle (id={id}): err={e}"),
        };
    }
    Ok(())
}
