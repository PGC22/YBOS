use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub prompt: String,
    pub max_tokens: usize,
    pub temperature: f32,
    pub top_p: f32,
    pub stop: Vec<String>,
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResponse {
    pub text: String,
    pub finish_reason: FinishReason,
    pub tokens_in: usize,
    pub tokens_out: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub logprob: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FinishReason {
    Stop,
    MaxTokens,
    StopSequence(String), // which stop sequence matched
    Error(String),        // error message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub backend: String,
    pub model_name: String,
    pub context_window: usize,
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum InferenceError {
    #[error("Model load error: {0}")]
    ModelLoad(String),
    #[error("Generation error: {0}")]
    Generation(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    #[error("Not implemented")]
    NotImplemented,
}
