use std::env;

use anyhow::Result;
use tokio::process::Command;

pub(crate) async fn inference(model_path: &str, prompt: &str) -> Result<String> {
    let output = Command::new("llama")
        .args([
            "--model",
            model_path,
            "--threads",
            &env::var("ECHOLOCATOR_THREADS").unwrap_or("4".to_string()),
            "--ctx-size",
            &env::var("ECHOLOCATOR_CTX_SIZE").unwrap_or("8192".to_string()),
            "--temp",
            &env::var("ECHOLOCATOR_TEMP").unwrap_or("0.8".to_string()),
            "--repeat-penalty",
            &env::var("ECHOLOCATOR_REPEAT_PENALTY").unwrap_or("1.2".to_string()),
            "--prompt",
            prompt,
            "--log-disable",
        ])
        .output()
        .await?;
    let completion = String::from_utf8(output.stdout)?;
    return Ok(completion);
}
