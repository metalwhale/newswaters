// See:
// - https://github.com/huggingface/candle/blob/8a82d62/candle-examples/examples/bert/main.rs
// - https://github.com/FlagOpen/FlagEmbedding/tree/b755dff/FlagEmbedding/llm_embedder#using-transformers

#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use anyhow::{Error, Result};
use candle_core::{Device, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::bert::{BertModel, DTYPE};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::{PaddingParams, Tokenizer, TruncationParams};

const MODEL_NAME: &str = "BAAI/llm-embedder";

pub(crate) struct Bert {
    model: BertModel,
    tokenizer: Tokenizer,
}

impl Bert {
    pub(crate) fn new() -> Result<Self> {
        let api = Api::new()?.repo(Repo::new(MODEL_NAME.to_string(), RepoType::Model));
        let config = serde_json::from_str(&std::fs::read_to_string(api.get("config.json")?)?)?;
        let model = BertModel::load(
            VarBuilder::from_pth(&api.get("pytorch_model.bin")?, DTYPE, &Device::Cpu)?,
            &config,
        )?;
        let tokenizer = Tokenizer::from_file(api.get("tokenizer.json")?).map_err(Error::msg)?;
        return Ok(Self { model, tokenizer });
    }

    pub(crate) fn embed(&mut self, sentence: &str) -> Result<Vec<f32>> {
        let tokenizer = self
            .tokenizer
            .with_padding(Some(PaddingParams::default()))
            .with_truncation(Some(TruncationParams::default()))
            .map_err(Error::msg)?;
        let tokens = tokenizer.encode(sentence, true).map_err(Error::msg)?.get_ids().to_vec();
        let token_ids = Tensor::new(&tokens[..], &self.model.device)?.unsqueeze(0)?;
        let token_type_ids = token_ids.zeros_like()?;
        let embedding = self
            .model
            .forward(&token_ids, &token_type_ids)?
            // CLS pooling (See: https://huggingface.co/BAAI/llm-embedder/blob/01fe9c0/1_Pooling/config.json#L3)
            .squeeze(0)?
            .get(0)?;
        let normalized_embedding = normalize_l2(&embedding)?;
        Ok(normalized_embedding.to_vec1::<f32>()?)
    }
}

fn normalize_l2(v: &Tensor) -> Result<Tensor> {
    Ok(v.broadcast_div(&v.sqr()?.sum_all()?.sqrt()?)?)
}
