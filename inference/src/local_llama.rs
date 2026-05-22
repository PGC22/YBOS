use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::num::NonZeroU32;
use async_trait::async_trait;
use futures::Stream;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaModel, AddBos, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::llama_backend::LlamaBackend;
use tokio_stream::wrappers::ReceiverStream;

use crate::trait_def::Inference;
use crate::types::{CompleteRequest, CompleteResponse, Token, ModelInfo, InferenceError, FinishReason};

// Since LlamaBackend is required for new_context, and it's not Clone, we need a better way.
// Let's just init it in a global and keep it.

pub struct LlamaParams {
    pub context_size: usize,
    pub n_threads: i32,
    pub n_gpu_layers: u32,
}

impl Default for LlamaParams {
    fn default() -> Self {
        Self {
            context_size: 8192,
            n_threads: num_cpus::get() as i32,
            n_gpu_layers: 0,
        }
    }
}

pub struct LocalLlama {
    model: Arc<LlamaModel>,
    backend: Arc<LlamaBackend>,
    params: LlamaParams,
}

impl LocalLlama {
    pub fn load(model_path: &Path, params: LlamaParams) -> Result<Self, InferenceError> {
        let backend = LlamaBackend::init().map_err(|e| InferenceError::ModelLoad(format!("Backend init failed: {:?}", e)))?;

        let model_params = LlamaModelParams::default()
            .with_n_gpu_layers(params.n_gpu_layers);

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| InferenceError::ModelLoad(e.to_string()))?;

        Ok(Self {
            model: Arc::new(model),
            backend: Arc::new(backend),
            params
        })
    }
}

#[async_trait]
impl Inference for LocalLlama {
    async fn complete(&self, req: CompleteRequest) -> Result<CompleteResponse, InferenceError> {
        let n_ctx = NonZeroU32::new(self.params.context_size as u32);
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(n_ctx)
            .with_n_threads(self.params.n_threads);

        let mut ctx = self.model.new_context(&self.backend, ctx_params)
            .map_err(|e| InferenceError::Generation(e.to_string()))?;

        let tokens_list = self.model.str_to_token(&req.prompt, AddBos::Always)
            .map_err(|e| InferenceError::Generation(format!("{:?}", e)))?;
        let tokens_in = tokens_list.len();

        let mut batch = LlamaBatch::new(self.params.context_size, 1);
        let last_index = tokens_list.len() as i32 - 1;
        for (i, token) in tokens_list.into_iter().enumerate() {
            let _ = batch.add(token, i as i32, &[0.into()], i as i32 == last_index);
        }

        ctx.decode(&mut batch).map_err(|e| InferenceError::Generation(e.to_string()))?;

        let mut samplers = vec![
            LlamaSampler::top_p(req.top_p, 1),
            LlamaSampler::temp(req.temperature),
        ];
        if let Some(seed) = req.seed {
            samplers.push(LlamaSampler::dist(seed as u32));
        }

        let mut sampler = LlamaSampler::chain_simple(samplers);

        let mut decoded_text = String::new();
        let mut tokens_out = 0;
        let mut n_cur = batch.n_tokens();

        while tokens_out < req.max_tokens {
            let token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(token);

            if self.model.is_eog_token(token) {
                return Ok(CompleteResponse {
                    text: decoded_text,
                    finish_reason: FinishReason::Stop,
                    tokens_in,
                    tokens_out: tokens_out as usize,
                });
            }

            let token_str = self.model.token_to_str(token, Special::Plaintext)
                .map_err(|e| InferenceError::Generation(e.to_string()))?;
            decoded_text.push_str(&token_str);
            tokens_out += 1;

            for stop_seq in &req.stop {
                if decoded_text.ends_with(stop_seq) {
                    return Ok(CompleteResponse {
                        text: decoded_text,
                        finish_reason: FinishReason::StopSequence,
                        tokens_in,
                        tokens_out: tokens_out as usize,
                    });
                }
            }

            batch.clear();
            let _ = batch.add(token, n_cur, &[0.into()], true);
            ctx.decode(&mut batch).map_err(|e| InferenceError::Generation(e.to_string()))?;
            n_cur += 1;
        }

        Ok(CompleteResponse {
            text: decoded_text,
            finish_reason: FinishReason::MaxTokens,
            tokens_in,
            tokens_out: tokens_out as usize,
        })
    }

    async fn complete_stream(
        &self,
        req: CompleteRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<Token, InferenceError>> + Send>>, InferenceError> {
        let model = self.model.clone();
        let backend = self.backend.clone();
        let context_size = self.params.context_size;
        let n_threads = self.params.n_threads;

        let (tx, rx) = tokio::sync::mpsc::channel(100);

        std::thread::spawn(move || {
            let n_ctx = NonZeroU32::new(context_size as u32);
            let ctx_params = LlamaContextParams::default()
                .with_n_ctx(n_ctx)
                .with_n_threads(n_threads);

            let mut ctx = match model.new_context(&backend, ctx_params) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.blocking_send(Err(InferenceError::Generation(e.to_string())));
                    return;
                }
            };

            let tokens_list = match model.str_to_token(&req.prompt, AddBos::Always) {
                Ok(t) => t,
                Err(e) => {
                    let _ = tx.blocking_send(Err(InferenceError::Generation(format!("{:?}", e))));
                    return;
                }
            };

            let mut batch = LlamaBatch::new(context_size, 1);
            let last_index = tokens_list.len() as i32 - 1;
            for (i, token) in tokens_list.into_iter().enumerate() {
                let _ = batch.add(token, i as i32, &[0.into()], i as i32 == last_index);
            }

            if let Err(e) = ctx.decode(&mut batch) {
                let _ = tx.blocking_send(Err(InferenceError::Generation(e.to_string())));
                return;
            }

            let mut samplers = vec![
                LlamaSampler::top_p(req.top_p, 1),
                LlamaSampler::temp(req.temperature),
            ];
            if let Some(seed) = req.seed {
                samplers.push(LlamaSampler::dist(seed as u32));
            }
            let mut sampler = LlamaSampler::chain_simple(samplers);

            let mut tokens_out = 0;
            let mut n_cur = batch.n_tokens();
            let mut current_text = String::new();

            while tokens_out < req.max_tokens {
                let token = sampler.sample(&ctx, batch.n_tokens() - 1);
                sampler.accept(token);

                if model.is_eog_token(token) {
                    break;
                }

                let token_str = match model.token_to_str(token, Special::Plaintext) {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.blocking_send(Err(InferenceError::Generation(e.to_string())));
                        break;
                    }
                };

                current_text.push_str(&token_str);
                if tx.blocking_send(Ok(Token { text: token_str, logprob: None })).is_err() {
                    break;
                }
                tokens_out += 1;

                let mut stop_found = false;
                for stop_seq in &req.stop {
                    if current_text.ends_with(stop_seq) {
                        stop_found = true;
                        break;
                    }
                }
                if stop_found {
                    break;
                }

                batch.clear();
                let _ = batch.add(token, n_cur, &[0.into()], true);
                if let Err(e) = ctx.decode(&mut batch) {
                    let _ = tx.blocking_send(Err(InferenceError::Generation(e.to_string())));
                    break;
                }
                n_cur += 1;
            }
        });

        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn model_info(&self) -> ModelInfo {
        ModelInfo {
            backend: "local-llama".into(),
            model_name: "GGUF Model".into(),
            context_window: self.params.context_size,
        }
    }
}
