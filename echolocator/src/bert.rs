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

pub(crate) fn build_model_and_tokenizer(model_name: String, tokenizer_name: String) -> Result<(BertModel, Tokenizer)> {
    let model_path = Api::new()?
        .repo(Repo::new(model_name, RepoType::Model))
        .get("model.safetensors")?;
    let tokenizer = Tokenizer::from_file(
        Api::new()?
            .repo(Repo::new(tokenizer_name, RepoType::Model))
            .get("tokenizer.json")?,
    )
    .map_err(Error::msg)?;
    let model = BertModel::new(
        unsafe { VarBuilder::from_mmaped_safetensors(&[model_path], DType::F32, &Device::Cpu)? },
        &Config::v2_base(),
    )?;
    Ok((model, tokenizer))
}

pub(crate) fn embed(model: BertModel, mut tokenizer: Tokenizer, sentence: &str) -> Result<Vec<f32>> {
    let tokenizer = tokenizer.with_padding(None).with_truncation(None).map_err(Error::msg)?;
    let tokens = tokenizer.encode(sentence, true).map_err(Error::msg)?.get_ids().to_vec();
    let token_ids = Tensor::new(&tokens[..], &model.device)?.unsqueeze(0)?;
    let embedding = model.forward(&token_ids)?;
    let (_sentences_num, tokens_num, _hidden_size) = embedding.dims3()?;
    let embedding = (embedding.sum(1)? / (tokens_num as f64))?.squeeze(0)?;
    Ok(embedding.to_vec1::<f32>()?)
}
