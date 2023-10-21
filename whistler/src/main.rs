use std::env;

use anyhow::Result;
use axum::{routing, Router};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new().route("/healthz", routing::get(|| async { "Ok" }));
    let port = env::var("WHISTLER_PORT").unwrap_or("3000".to_string());
    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
