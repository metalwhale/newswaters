pub(crate) mod hacker_news;
pub(crate) mod inference;
pub(crate) mod search_engine;

use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Item {
    pub id: i32,
    pub deleted: Option<bool>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub by: Option<String>,
    pub time: Option<i64>,
    pub text: Option<String>,
    pub dead: Option<bool>,
    pub parent: Option<i32>,
    pub poll: Option<i32>,
    #[allow(dead_code)]
    pub kids: Option<Vec<i32>>,
    pub url: Option<String>,
    pub score: Option<i32>,
    pub title: Option<String>,
    #[allow(dead_code)]
    pub parts: Option<Vec<i32>>,
    pub descendants: Option<i32>,
}

pub(crate) enum ItemUrl {
    Finished { html: String, text: String },
    Skipped { note: String },
    Canceled { note: String },
}

#[derive(Deserialize)]
pub(crate) struct Analysis {
    pub item_id: i32,
    pub keyword: Option<String>,
}
