use std::env;

use anyhow::Result;
use serde::Serialize;

use crate::{
    command,
    repository::Repository,
    service::{hacker_news, inference, search_engine, Analysis},
};

pub(crate) async fn analyze_story_texts(mut repo: Repository) -> Result<()> {
    let texts_num: usize = env::var("JOB_ANALYZE_STORY_TEXTS_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    let top_story_ids = hacker_news::get_top_story_ids().await?;
    let mut analyses = repo.find_keyword_missing_analyses(&top_story_ids)?;
    // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
    //   as `LIMIT` doesn't maintain the order of top stories' ids.
    analyses.truncate(texts_num);
    if env::var("JOB_ANALYZE_ADDITIONAL_TEXTS").is_ok() && analyses.len() < texts_num {
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
            "[INFO] main.analyze_story_texts (id={}): text.len={}, keyword.len={}, elapsed_time={:?}",
            id,
            text.len(),
            keyword.len(),
            start_time.elapsed()
        );
        repo.insert_analysis(Analysis {
            item_id: id,
            keyword: Some(keyword),
            text_passage: None,
        })?;
    }
    Ok(())
}

pub(crate) async fn analyze_comment_texts(mut repo: Repository) -> Result<()> {
    let min_len: usize = env::var("JOB_ANALYZE_COMMENT_TEXT_MIN_LEN")
        .unwrap_or("120".to_string())
        .parse()?;
    let max_len: usize = env::var("JOB_ANALYZE_COMMENT_TEXT_MAX_LEN")
        .unwrap_or("4800".to_string())
        .parse()?;
    let texts_num: usize = env::var("JOB_ANALYZE_COMMENT_TEXTS_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    let analyses = repo.find_text_passage_missing_analyses(min_len, texts_num)?;
    for (id, mut text) in analyses {
        text.truncate(max_len);
        let start_time = std::time::Instant::now();
        let anchor_passage = match inference::instruct_comment_anchor_passage(&text).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_comment_anchor_passage (id={id}): err={e}");
                continue;
            }
        };
        let entailment_passage = match inference::instruct_entailment_passage(&anchor_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_entailment_passage (id={id}): err={e}");
                continue;
            }
        };
        let contradiction_passage = match inference::instruct_contradiction_passage(&anchor_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_contradiction_passage (id={id}): err={e}");
                continue;
            }
        };
        // TODO: Generate a genuinely irrelevant passage
        let irrelevance_passage = match inference::instruct_random_passage(&contradiction_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_random_passage (id={id}): err={e}");
                continue;
            }
        };
        println!(
            "[INFO] main.analyze_comment_texts (id={}): text.len={}, \
                anchor_passage.len={}, entailment_passage.len={}, contradiction_passage.len={}, irrelevance_passage.len={}, \
                elapsed_time={:?}",
            id,
            text.len(),
            anchor_passage.len(),
            entailment_passage.len(),
            contradiction_passage.len(),
            irrelevance_passage.len(),
            start_time.elapsed()
        );
        let text_passage = serde_json::to_string(&Passage {
            anchor: vec![anchor_passage],
            entailment: vec![entailment_passage],
            contradiction: vec![contradiction_passage],
            irrelevance: vec![irrelevance_passage],
            subject: vec![],
        })?;
        repo.insert_analysis(Analysis {
            item_id: id,
            keyword: None,
            text_passage: Some(text_passage),
        })?;
    }
    Ok(())
}

pub(crate) async fn analyze_summaries(mut repo: Repository) -> Result<()> {
    let summaries_num: usize = env::var("JOB_ANALYZE_SUMMARIES_NUM")
        .unwrap_or("30".to_string())
        .parse()?;
    let top_story_ids = hacker_news::get_top_story_ids().await?;
    let mut analyses = repo.find_summary_passage_missing_analyses(&top_story_ids)?;
    // NOTE: We must use `truncate` function here instead of `LIMIT` in the query,
    //   as `LIMIT` doesn't maintain the order of top stories' ids.
    analyses.truncate(summaries_num);
    if env::var("JOB_ANALYZE_ADDITIONAL_SUMMARIES").is_ok() && analyses.len() < summaries_num {
        let mut additional_items =
            repo.find_summary_passage_missing_analyses_excluding(&top_story_ids, summaries_num - analyses.len())?;
        analyses.append(&mut additional_items);
    }
    for (id, summary) in analyses {
        let start_time = std::time::Instant::now();
        let anchor_passage = match inference::instruct_summary_anchor_passage(&summary).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_summary_anchor_passage (id={id}): err={e}");
                continue;
            }
        };
        let entailment_passage = match inference::instruct_entailment_passage(&anchor_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_entailment_passage (id={id}): err={e}");
                continue;
            }
        };
        let contradiction_passage = match inference::instruct_contradiction_passage(&anchor_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_contradiction_passage (id={id}): err={e}");
                continue;
            }
        };
        // TODO: Generate a genuinely irrelevant passage
        let irrelevance_passage = match inference::instruct_random_passage(&contradiction_passage).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_random_passage (id={id}): err={e}");
                continue;
            }
        };
        let subject_passage = match inference::instruct_subject_passage(&summary).await {
            Ok(passage) => passage,
            Err(e) => {
                println!("[ERR] inference.instruct_subject_passage (id={id}): err={e}");
                continue;
            }
        };
        println!(
            "[INFO] main.analyze_summaries (id={}): summary.len={}, \
                anchor_passage.len={}, entailment_passage.len={}, contradiction_passage.len={}, irrelevance_passage.len={}, \
                subject_passage.len={}, \
                elapsed_time={:?}",
            id,
            summary.len(),
            anchor_passage.len(),
            entailment_passage.len(),
            contradiction_passage.len(),
            irrelevance_passage.len(),
            subject_passage.len(),
            start_time.elapsed()
        );
        let summary_passage = serde_json::to_string(&Passage {
            anchor: vec![anchor_passage],
            entailment: vec![entailment_passage],
            contradiction: vec![contradiction_passage],
            irrelevance: vec![irrelevance_passage],
            subject: subject_passage.split("\n").map(str::to_string).collect(),
        })?;
        repo.update_analysis(id, summary_passage)?;
    }
    Ok(())
}

#[derive(Serialize)]
struct Passage {
    anchor: Vec<String>,
    entailment: Vec<String>,
    contradiction: Vec<String>,
    irrelevance: Vec<String>,
    subject: Vec<String>,
}

pub(crate) async fn embed_keywords(mut repo: Repository) -> Result<()> {
    let collection_name = env::var("SEARCH_ENGINE_VECTOR_KEYWORD_COLLECTION_NAME")?;
    let keywords_num: usize = env::var("JOB_EMBED_KEYWORDS_NUM")
        .unwrap_or("1000000".to_string())
        .parse()?;
    let chunk_size: usize = env::var("JOB_CHUNK_SIZE").unwrap_or("50".to_string()).parse()?;
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
    Ok(())
}
