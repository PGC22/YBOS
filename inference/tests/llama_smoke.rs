#[cfg(feature = "local_llama")]
use ybos_inference::{LocalLlama, Inference, CompleteRequest, LlamaParams};
#[cfg(feature = "local_llama")]
use futures::StreamExt;
#[cfg(feature = "local_llama")]
use std::path::PathBuf;
#[cfg(feature = "local_llama")]
use std::fs;

#[cfg(feature = "local_llama")]
const MODEL_URL: &str = "https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf";
#[cfg(feature = "local_llama")]
const MODEL_FILENAME: &str = "tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf";

#[cfg(feature = "local_llama")]
fn download_model() -> PathBuf {
    let mut model_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    model_dir.push("../target/test-models");
    fs::create_dir_all(&model_dir).expect("Failed to create test-models directory");

    let model_path = model_dir.join(MODEL_FILENAME);
    if !model_path.exists() {
        eprintln!("Downloading model {}...", MODEL_FILENAME);
        let mut response = reqwest::blocking::get(MODEL_URL).expect("Failed to download model");
        let mut file = fs::File::create(&model_path).expect("Failed to create model file");
        std::io::copy(&mut response, &mut file).expect("Failed to save model file");
        eprintln!("Model downloaded to {:?}", model_path);
    }
    model_path
}

#[cfg(feature = "local_llama")]
#[tokio::test]
async fn test_llama_complete() {
    let model_path = tokio::task::spawn_blocking(download_model).await.expect("Download failed");
    let llama = LocalLlama::load(&model_path, LlamaParams::default()).expect("Failed to load model");

    let req = CompleteRequest {
        prompt: "<|system|>\nYou are a helpful assistant.</s>\n<|user|>\nWhat is 2+2? Answer in one word.</s>\n<|assistant|>\n".into(),
        max_tokens: 16,
        temperature: 0.1,
        top_p: 0.9,
        stop: vec![],
        seed: Some(42),
    };

    let res = llama.complete(req).await.expect("Inference failed");
    println!("Response: {}", res.text);
    assert!(!res.text.is_empty());
    assert!(res.tokens_out > 0);
}

#[cfg(feature = "local_llama")]
#[tokio::test]
async fn test_llama_complete_stream() {
    let model_path = tokio::task::spawn_blocking(download_model).await.expect("Download failed");
    let llama = LocalLlama::load(&model_path, LlamaParams::default()).expect("Failed to load model");

    let req = CompleteRequest {
        prompt: "<|system|>\nYou are a helpful assistant.</s>\n<|user|>\nWhat is 2+2? Answer in one word.</s>\n<|assistant|>\n".into(),
        max_tokens: 16,
        temperature: 0.1,
        top_p: 0.9,
        stop: vec![],
        seed: Some(42),
    };

    let mut stream = llama.complete_stream(req).await.expect("Inference failed");
    let mut tokens_count = 0;
    let mut full_text = String::new();

    while let Some(res) = stream.next().await {
        let token = res.expect("Token error");
        full_text.push_str(&token.text);
        tokens_count += 1;
    }

    println!("Streamed Response: {}", full_text);
    assert!(tokens_count > 0);
}
