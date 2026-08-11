use crate::config::Config;
use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone)]
pub struct EmbeddingClient {
    http: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    #[allow(dead_code)]
    pub dim: usize,
}

impl EmbeddingClient {
    pub fn from_config(config: &Config) -> Option<Self> {
        let base_url = config.embeddings_base_url.clone()?;
        let model = config.embeddings_model.clone()?;
        Some(Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model,
            api_key: config.embeddings_api_key.clone(),
            dim: config.embedding_dim,
        })
    }

    pub async fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/embeddings", self.base_url);
        let mut req = self.http.post(&url).json(&json!({
            "model": self.model,
            "input": texts,
        }));
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.context("embeddings request")?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow!("embeddings HTTP {status}: {body}"));
        }
        #[derive(Deserialize)]
        struct EmbResp {
            data: Vec<EmbData>,
        }
        #[derive(Deserialize)]
        struct EmbData {
            embedding: Vec<f32>,
            index: usize,
        }
        let body: EmbResp = resp.json().await.context("decode embeddings")?;
        let mut out = vec![Vec::new(); texts.len()];
        for d in body.data {
            if d.index < out.len() {
                out[d.index] = d.embedding;
            }
        }
        Ok(out)
    }
}

pub fn f32_to_blob(v: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(v.len() * 4);
    for f in v {
        out.extend_from_slice(&f.to_le_bytes());
    }
    out
}
