use std::{env, time::Duration};

use anyhow::Result;
use tokio::{self};

use crate::{
    command,
    repository::Repository,
    service::{hacker_news, inference, search_engine, Analysis},
};

pub(crate) async fn analyze_texts(mut repo: Repository, is_job: bool) -> Result<()> {
    let texts_num: usize = env::var("SKIMMER_ANALYZE_TEXTS_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    loop {
        let top_story_ids = hacker_news::get_top_story_ids().await?;
        let mut analyses = repo.find_keyword_missing_analyses(&top_story_ids)?;
        // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
        //   as `LIMIT` doesn't maintain the order of top stories' ids.
        analyses.truncate(texts_num);
        if env::var("SKIMMER_ANALYZE_ADDITIONAL_TEXTS").is_ok() && analyses.len() < texts_num {
            let mut additional_items =
                repo.find_keyword_missing_analyses_excluding(&top_story_ids, texts_num - analyses.len())?;
            analyses.append(&mut additional_items);
        }
        for (id, title, text, url_text) in analyses {
            let text = if let Some(text) = text {
                text
            } else if let Some(url_text) = url_text {
                command::shorten_text(&url_text)?
            } else {
                continue;
            };
            let start_time = std::time::Instant::now();
            let keyword = match inference::instruct_keyword(&title, &text).await {
                Ok(keyword) => keyword,
                Err(e) => {
                    println!("[ERR] inference.instruct_keyword (id={id}): err={e}");
                    continue;
                }
            };
            println!(
                "[INFO] main.analyze_texts (id={}): text.len={}, keyword.len={}, elapsed_time={:?}",
                id,
                text.len(),
                keyword.len(),
                start_time.elapsed()
            );
            repo.insert_analysis(Analysis {
                item_id: id,
                keyword: Some(keyword),
            })?;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

pub(crate) async fn embed_keywords(mut repo: Repository, is_job: bool) -> Result<()> {
    let collection_name = env::var("SEARCH_ENGINE_VECTOR_KEYWORD_COLLECTION_NAME")?;
    let keywords_num: usize = env::var("SKIMMER_EMBED_KEYWORDS_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let chunk_size: usize = env::var("SKIMMER_CHUNK_SIZE").unwrap_or("50".to_string()).parse()?;
    loop {
        let keyword_existing_ids = repo.find_keyword_existing_analyses(keywords_num)?;
        let embedding_missing_ids = search_engine::find_missing(collection_name.clone(), keyword_existing_ids).await?;
        for chunk in embedding_missing_ids.chunks(chunk_size) {
            let analysis_keywords = repo.find_analysis_keywords(chunk)?;
            for (id, keyword) in analysis_keywords {
                let embedding = inference::embed(keyword).await?;
                search_engine::upsert(collection_name.clone(), id, embedding).await?;
                println!("[INFO] main.embed_keywords (id={})", id);
            }
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

pub(crate) async fn analyze_summaries(mut repo: Repository, is_job: bool) -> Result<()> {
    let summaries_num: usize = env::var("SKIMMER_ANALYZE_SUMMARIES_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    loop {
        let top_story_ids = hacker_news::get_top_story_ids().await?;
        let mut analyses = repo.find_summary_query_missing_analyses(&top_story_ids)?;
        // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
        //   as `LIMIT` doesn't maintain the order of top stories' ids.
        analyses.truncate(summaries_num);
        if env::var("SKIMMER_ANALYZE_ADDITIONAL_SUMMARIES").is_ok() && analyses.len() < summaries_num {
            let mut additional_items =
                repo.find_summary_query_missing_analyses_excluding(&top_story_ids, summaries_num - analyses.len())?;
            analyses.append(&mut additional_items);
        }
        for (id, summary) in analyses {
            let start_time = std::time::Instant::now();
            let summary_query = match {
                let max_retry_count = 10;
                let mut retry_count = 0;
                // TODO: Find a better way to constrain output with the correct format
                loop {
                    let summary_query = match inference::instruct_summary_query(&summary).await {
                        Ok(summary_query) => summary_query,
                        Err(e) => {
                            println!("[ERR] inference.instruct_summary_query (id={id}): err={e}");
                            continue;
                        }
                    };
                    match serde_json::from_str::<serde_json::Value>(&summary_query) {
                        Ok(_) => break Ok(summary_query),
                        Err(e) => {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                            retry_count += 1;
                            if retry_count >= max_retry_count {
                                break Err(e);
                            }
                            continue;
                        }
                    }
                }
            } {
                Ok(summary_query) => summary_query,
                Err(e) => {
                    println!("[ERR] inference.analyze_summaries (id={id}): err={e}");
                    continue;
                }
            };
            println!(
                "[INFO] main.analyze_summaries (id={}): summary.len={}, summary_query.len={}, elapsed_time={:?}",
                id,
                summary.len(),
                summary_query.len(),
                start_time.elapsed()
            );
            repo.update_analysis(id, summary_query)?;
        }
        if is_job {
            break Ok(());
        } else {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
