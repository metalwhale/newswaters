// See: https://github.com/huggingface/candle/blob/0ec5ebc/candle-examples/examples/jina-bert/main.rs

#[cfg(feature = "mkl")]
extern crate intel_mkl_src;

#[cfg(feature = "accelerate")]
extern crate accelerate_src;

use anyhow::{Error, Result};
use candle_core::{DType, Device, Module, Tensor};
use candle_nn::VarBuilder;
use candle_transformers::models::jina_bert::{BertModel, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;

const MODEL_NAME: &str = "jinaai/jina-embeddings-v2-base-en";
const TOKENIZER_NAME: &str = "sentence-transformers/all-MiniLM-L6-v2";

#[derive(Clone)]
pub(crate) struct Bert {
    model: BertModel,
    tokenizer: Tokenizer,
}

impl Bert {
    pub(crate) fn new() -> Result<Self> {
        let model_path = Api::new()?
            .repo(Repo::new(MODEL_NAME.to_string(), RepoType::Model))
            .get("model.safetensors")?;
        let tokenizer = Tokenizer::from_file(
            Api::new()?
                .repo(Repo::new(TOKENIZER_NAME.to_string(), RepoType::Model))
                .get("tokenizer.json")?,
        )
        .map_err(Error::msg)?;
        let model = BertModel::new(
            unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &Device::Cpu)? },
            &Config::v2_base(),
        )?;
        return Ok(Self { model, tokenizer });
    }

    pub(crate) fn embed(&mut self, sentence: &str) -> Result<Vec<f32>> {
        let tokenizer = self
            .tokenizer
            .with_padding(None)
            .with_truncation(None)
            .map_err(Error::msg)?;
        let tokens = tokenizer.encode(sentence, true).map_err(Error::msg)?.get_ids().to_vec();
        let token_ids = Tensor::new(&tokens[..], &self.model.device)?.unsqueeze(0)?;
        let embedding = self.model.forward(&token_ids)?;
        let (_sentences_num, tokens_num, _hidden_size) = embedding.dims3()?;
        let embedding = (embedding.sum(1)? / (tokens_num as f64))?.squeeze(0)?;
        Ok(embedding.to_vec1::<f32>()?)
    }
}
