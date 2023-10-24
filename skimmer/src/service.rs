use anyhow::{bail, Result};
use chromiumoxide::browser::{Browser, BrowserConfig};
use futures::StreamExt;
use html2text::{self, render::text_renderer::TrivialDecorator};
use reqwest::header::CONTENT_TYPE;
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

// See: https://github.com/HackerNews/API/tree/38154ba#max-item-id
pub(crate) async fn get_max_item() -> Result<i32> {
    let response = reqwest::get("https://hacker-news.firebaseio.com/v0/maxitem.json?print=pretty").await?;
    Ok(response.text().await?.trim().parse()?)
}

// See: https://github.com/HackerNews/API/tree/38154ba#items
pub(crate) async fn get_item(id: i32) -> Result<Item> {
    let response = reqwest::get(format!(
        "https://hacker-news.firebaseio.com/v0/item/{}.json?print=pretty",
        id
    ))
    .await?;
    let item = response.json::<Item>().await?;
    Ok(item)
}

pub(crate) enum ItemUrl {
    Finished { html: String, text: String },
    Skipped { note: String },
    Canceled { note: String },
}

pub(crate) async fn get_item_url(url: &str) -> Result<ItemUrl> {
    let response = reqwest::get(url).await?;
    let skipping_note = match response.headers().get(CONTENT_TYPE) {
        Some(value) => match value.to_str() {
            Ok(t) => {
                if t.to_lowercase().contains("pdf") {
                    Some(format!("Skipped: {}", t))
                } else {
                    None
                }
            }
            Err(_) => None,
        },
        None => None,
    };
    if let Some(note) = skipping_note {
        return Ok(ItemUrl::Skipped { note });
    } else {
        let config = match BrowserConfig::builder()
            .incognito()
            // https://github.com/puppeteer/puppeteer/issues/1825#issuecomment-651755428
            .no_sandbox()
            .arg("--disable-gpu")
            .arg("--single-process")
            .arg("--no-zygote")
            .build()
        {
            Ok(config) => config,
            Err(e) => bail!(e),
        };
        let (mut browser, mut handler) = Browser::launch(config).await?;
        let handle = tokio::spawn(async move {
            while let Some(h) = handler.next().await {
                if h.is_err() {
                    break;
                }
            }
        });
        let page = browser.new_page(url).await?;
        let html = page.content().await?;
        browser.close().await?;
        handle.await?;
        let text = match std::panic::catch_unwind(|| {
            html2text::from_read_with_decorator(html.as_bytes(), std::usize::MAX, TrivialDecorator::new())
        }) {
            Ok(text) => text,
            Err(_) => bail!("An error occurred in html2text::from_read_with_decorator"),
        };
        return Ok(ItemUrl::Finished { html, text });
    }
}
