use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use async_trait::async_trait;
use futures::Stream;
use tokio_stream::wrappers::ReceiverStream;
use crate::trait_def::Inference;
use crate::types::{CompleteRequest, CompleteResponse, Token, ModelInfo, InferenceError, FinishReason};

pub struct MockInference {
    canned_responses: Vec<String>,
    counter: Arc<AtomicUsize>,
}

impl MockInference {
    pub fn new(canned_responses: Vec<String>) -> Self {
        Self {
            canned_responses,
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl Inference for MockInference {
    async fn complete(&self, req: CompleteRequest) -> Result<CompleteResponse, InferenceError> {
        if self.canned_responses.is_empty() {
            return Err(InferenceError::Generation("No canned responses".into()));
        }
        let idx = self.counter.fetch_add(1, Ordering::SeqCst) % self.canned_responses.len();

        let full_text = &self.canned_responses[idx];
        let words: Vec<&str> = full_text.split_whitespace().collect();

        let (text, finish_reason, tokens_out) = if words.len() > req.max_tokens {
            (
                words[..req.max_tokens].join(" "),
                FinishReason::MaxTokens,
                req.max_tokens,
            )
        } else {
            (full_text.clone(), FinishReason::Stop, words.len())
        };

        Ok(CompleteResponse {
            text,
            finish_reason,
            tokens_in: 0,
            tokens_out,
        })
    }

    async fn complete_stream(
        &self,
        req: CompleteRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Token, InferenceError>> + Send>>, InferenceError> {
        if self.canned_responses.is_empty() {
            return Err(InferenceError::Generation("No canned responses".into()));
        }
        let idx = self.counter.fetch_add(1, Ordering::SeqCst) % self.canned_responses.len();
        let full_text = self.canned_responses[idx].clone();
        let words: Vec<String> = full_text.split_whitespace().map(|s| s.to_string()).collect();
        let max_tokens = req.max_tokens;

        let (tx, rx) = tokio::sync::mpsc::channel(10);

        tokio::spawn(async move {
            for (i, word) in words.into_iter().enumerate() {
                if i >= max_tokens {
                    break;
                }
                // Y6 fix: drop manual " " prefix on subsequent tokens.
                // Patterns matches LocalLlama which emits raw tokenizer output.
                let token_text = word;
                if tx.send(Ok(Token { text: token_text, logprob: None })).await.is_err() {
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            backend: "mock".into(),
            model_name: "mock-v1".into(),
            context_window: 4096,
        }
    }
}
