use std::env;

use anyhow::Result;
use chrono::{DateTime, Local};
use diesel::{pg::Pg, prelude::*, sql_types::*};

use super::Repository;
use crate::{schema::analyses, service::Analysis};

impl Repository {
    //////////////////////
    // Analyze story texts
    //////////////////////
    pub(crate) fn find_keyword_missing_analyses(
        &mut self,
        ids: &[i32],
    ) -> Result<Vec<(i32, String, Option<String>, Option<String>)>> {
        let keyword_missing_analyses = diesel::sql_query(format!(
            "SELECT id, title, items.text, item_urls.text AS url_text \
            FROM unnest(ARRAY[{}]) AS s(i) \
            JOIN items ON s.i = items.id \
            JOIN item_urls ON s.i = item_urls.item_id \
            LEFT JOIN analyses ON s.i = analyses.item_id \
            WHERE title IS NOT NULL AND (items.text IS NOT NULL OR item_urls.text IS NOT NULL) AND keyword IS NULL",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
        ))
        .get_results::<KeywordMissingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.title, r.text, r.url_text))
        .collect();
        return Ok(keyword_missing_analyses);
    }

    pub(crate) fn find_keyword_missing_analyses_excluding(
        &mut self,
        ids: &[i32],
        limit: usize,
    ) -> Result<Vec<(i32, String, Option<String>, Option<String>)>> {
        let item_url_condition = if env::var("JOB_FIND_ANALYSES_FOLLOW_SUMMARIES").is_ok() {
            "(item_urls.text IS NOT NULL AND item_urls.summary IS NOT NULL)"
        } else {
            "item_urls.text IS NOT NULL"
        };
        let keyword_missing_analyses = diesel::sql_query(format!(
            "SELECT id, title, items.text, item_urls.text AS url_text \
            FROM items \
            JOIN item_urls ON items.id = item_urls.item_id \
            LEFT JOIN analyses ON items.id = analyses.item_id \
            WHERE title IS NOT NULL AND (items.text IS NOT NULL OR {}) AND keyword IS NULL AND id NOT IN ({}) \
            ORDER BY id DESC LIMIT {}",
            item_url_condition,
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", "),
            limit
        ))
        .get_results::<KeywordMissingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.title, r.text, r.url_text))
        .collect();
        return Ok(keyword_missing_analyses);
    }

    pub(crate) fn find_keyword_existing_analyses(&mut self, limit: usize) -> Result<Vec<i32>> {
        let keyword_existing_analyses = diesel::sql_query(format!(
            "SELECT item_id \
            FROM analyses \
            WHERE keyword IS NOT NULL \
            ORDER BY item_id DESC LIMIT {}",
            limit
        ))
        .get_results::<KeywordExistingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.item_id))
        .collect();
        return Ok(keyword_existing_analyses);
    }

    pub(crate) fn find_analysis_keywords(&mut self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let analysis_keywords = diesel::sql_query(format!(
            "SELECT item_id, keyword \
            FROM unnest(ARRAY[{}]) AS s(i) \
            JOIN analyses ON s.i = analyses.item_id \
            WHERE keyword IS NOT NULL",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
        ))
        .get_results::<AnalysisKeywordRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.item_id, r.keyword))
        .collect();
        return Ok(analysis_keywords);
    }

    ////////////////////////
    // Analyze comment texts
    ////////////////////////
    pub(crate) fn find_text_passage_missing_analyses(
        &mut self,
        min_len: usize,
        limit: usize,
    ) -> Result<Vec<(i32, String)>> {
        let text_passage_missing_analyses = diesel::sql_query(format!(
            "SELECT id, text \
            FROM items \
            LEFT JOIN analyses ON items.id = analyses.item_id \
            WHERE type = 'comment' AND text IS NOT NULL AND length(text) >= {} \
            AND text_passage IS NULL \
            ORDER BY id DESC LIMIT {}",
            min_len, limit
        ))
        .get_results::<TextPassageMissingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.text))
        .collect();
        return Ok(text_passage_missing_analyses);
    }

    ////////////////////
    // Analyze summaries
    ////////////////////
    pub(crate) fn find_summary_passage_missing_analyses(&mut self, ids: &[i32]) -> Result<Vec<(i32, String)>> {
        let summary_passage_missing_analyses = diesel::sql_query(format!(
            "SELECT s.i AS id, summary \
            FROM unnest(ARRAY[{}]) AS s(i) \
            JOIN item_urls ON s.i = item_urls.item_id \
            LEFT JOIN analyses ON s.i = analyses.item_id \
            WHERE summary IS NOT NULL AND summary_passage IS NULL",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", ")
        ))
        .get_results::<SummaryPassageMissingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.summary))
        .collect();
        return Ok(summary_passage_missing_analyses);
    }

    pub(crate) fn find_summary_passage_missing_analyses_excluding(
        &mut self,
        ids: &[i32],
        limit: usize,
    ) -> Result<Vec<(i32, String)>> {
        let summary_passage_missing_analyses = diesel::sql_query(format!(
            "SELECT item_urls.item_id AS id, summary \
            FROM item_urls \
            LEFT JOIN analyses ON item_urls.item_id = analyses.item_id \
            WHERE summary IS NOT NULL AND summary_passage IS NULL AND item_urls.item_id NOT IN ({}) \
            ORDER BY id DESC LIMIT {}",
            ids.iter().map(|i| i.to_string()).collect::<Vec<String>>().join(", "),
            limit
        ))
        .get_results::<SummaryPassageMissingAnalysisRecord>(&mut self.connection)?
        .into_iter()
        .map(|r| (r.id, r.summary))
        .collect();
        return Ok(summary_passage_missing_analyses);
    }

    /////////
    // Common
    /////////
    pub(crate) fn insert_analysis(&mut self, analysis: Analysis) -> Result<()> {
        let analysis_record = InsertAnalysisRecord {
            item_id: analysis.item_id,
            keyword: analysis.keyword,
            text_passage: analysis.text_passage,
            summary_passage: analysis.summary_passage,
            created_at: Local::now(),
            updated_at: Local::now(),
        };
        diesel::insert_into(analyses::table)
            .values(&analysis_record)
            .returning(InsertAnalysisRecord::as_returning())
            .get_result(&mut self.connection)?;
        Ok(())
    }
}

//////////////////////
// Analyze story texts
//////////////////////
#[derive(QueryableByName)]
struct KeywordMissingAnalysisRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Nullable<Text>)]
    text: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    url_text: Option<String>,
}

#[derive(QueryableByName)]
struct KeywordExistingAnalysisRecord {
    #[diesel(sql_type = Integer)]
    item_id: i32,
}

#[derive(QueryableByName)]
struct AnalysisKeywordRecord {
    #[diesel(sql_type = Integer)]
    item_id: i32,
    #[diesel(sql_type = Text)]
    keyword: String,
}

////////////////////////
// Analyze comment texts
////////////////////////
#[derive(QueryableByName)]
struct TextPassageMissingAnalysisRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    text: String,
}

////////////////////
// Analyze summaries
////////////////////
#[derive(QueryableByName)]
struct SummaryPassageMissingAnalysisRecord {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    summary: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = analyses)]
#[diesel(check_for_backend(Pg))]
struct UpdateAnalysisRecord {
    summary_passage: Option<String>,
    updated_at: DateTime<Local>,
}

/////////
// Common
/////////
#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = analyses)]
#[diesel(check_for_backend(Pg))]
struct InsertAnalysisRecord {
    item_id: i32,
    keyword: Option<String>,
    text_passage: Option<String>,
    summary_passage: Option<String>,
    created_at: DateTime<Local>,
    updated_at: DateTime<Local>,
}
