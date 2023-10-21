use std::env;

use anyhow::{Ok, Result};
use axum::{routing, Router};
use futures_util::StreamExt;
use tokio::{fs::File, io::AsyncWriteExt};

#[tokio::main]
async fn main() -> Result<()> {
    let model_path = "/usr/src/models/mistral-7b-instruct-v0.1.Q4_K_M.gguf";
    download_model_file(model_path).await?;
    let app = Router::new().route("/healthz", routing::get(|| async { "Ok" }));
    let port = env::var("ECHOLOCATOR_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn download_model_file(model_path: &str) -> Result<()> {
    if std::path::Path::new(model_path).exists() {
        return Ok(());
    }
    let model_url = "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.Q4_K_M.gguf";
    let mut file = File::create(model_path).await?;
    let mut stream = reqwest::get(model_url).await?.bytes_stream();
    while let Some(chunk) = stream.next().await {
        file.write_all(&chunk?).await?;
    }
    file.flush().await?;
    Ok(())
}
