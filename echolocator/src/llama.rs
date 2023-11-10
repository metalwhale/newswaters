use std::env;

use anyhow::{Context, Result};
use futures_util::StreamExt;
use tokio::process::Command;
use tokio::{fs::File, io::AsyncWriteExt};

const MODEL_DIR: &str = "/usr/src/models";
const MODEL_URL: &str =
    "https://huggingface.co/TheBloke/Mistral-7B-Instruct-v0.1-GGUF/resolve/main/mistral-7b-instruct-v0.1.Q5_K_M.gguf";
const INSTRUCT_TEMPLATE: &str = "<s>[INST] {instruction} [/INST]";

#[derive(Clone)]
pub(crate) struct Llama {
    model_path: String,
    instruct_template: String,
}

impl Llama {
    pub(crate) async fn new() -> Result<Self> {
        std::fs::create_dir_all(MODEL_DIR)?;
        let model_name = MODEL_URL
            .split("/")
            .last()
            .context(format!("model_url={MODEL_URL}"))?
            .to_string();
        let model_path = format!("{MODEL_DIR}/{model_name}");
        if !std::path::Path::new(&model_path).exists() {
            let mut file = File::create(&model_path).await?;
            let mut stream = reqwest::get(MODEL_URL).await?.bytes_stream();
            while let Some(chunk) = stream.next().await {
                file.write_all(&chunk?).await?;
            }
            file.flush().await?;
        }
        return Ok(Self {
            model_path,
            instruct_template: INSTRUCT_TEMPLATE.to_string(),
        });
    }

    pub(crate) async fn inference(&self, instruction: &str) -> Result<String> {
        let prompt = self.instruct_template.replace("{instruction}", instruction);
        let output = Command::new("llama")
            .args([
                "--model",
                &self.model_path,
                "--threads",
                &env::var("ECHOLOCATOR_THREADS").unwrap_or("4".to_string()),
                "--ctx-size",
                &env::var("ECHOLOCATOR_CTX_SIZE").unwrap_or("8192".to_string()),
                "--temp",
                &env::var("ECHOLOCATOR_TEMP").unwrap_or("0.8".to_string()),
                "--repeat-penalty",
                &env::var("ECHOLOCATOR_REPEAT_PENALTY").unwrap_or("1.2".to_string()),
                "--prompt",
                &prompt,
                "--log-disable",
            ])
            .output()
            .await?;
        let completion = String::from_utf8(output.stdout)?
            // TODO: Prevent the output of the prompt rather than having to manually remove it from the completion
            .replace(
                // prompt lacks both BOS and EOS markers in the completion
                &prompt.replace("<s>", "").replace("</s>", ""),
                "",
            )
            .trim()
            .to_string();
        return Ok(completion);
    }
}
