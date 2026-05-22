use ybos_inference::{MockInference, Inference, CompleteRequest};
use futures::StreamExt;

#[tokio::test]
async fn test_mock_complete() {
    let mock = MockInference::new(vec!["one two three".into(), "four five six".into()]);

    let req = CompleteRequest {
        prompt: "test".into(),
        max_tokens: 10,
        temperature: 0.7,
        top_p: 0.9,
        stop: vec![],
        seed: None,
    };

    let res = mock.complete(req).await.unwrap();
    assert_eq!(res.text, "one two three");
}

#[tokio::test]
async fn test_mock_complete_stream() {
    let mock = MockInference::new(vec!["one two three".into(), "four five six".into()]);

    // Skip first canned response
    let _ = mock.complete(CompleteRequest {
        prompt: "test".into(),
        max_tokens: 10,
        temperature: 0.7,
        top_p: 0.9,
        stop: vec![],
        seed: None,
    }).await.unwrap();

    let req = CompleteRequest {
        prompt: "test".into(),
        max_tokens: 10,
        temperature: 0.7,
        top_p: 0.9,
        stop: vec![],
        seed: None,
    };

    let mut stream = mock.complete_stream(req).await.unwrap();
    let mut tokens = Vec::new();
    while let Some(res) = stream.next().await {
        tokens.push(res.unwrap().text.trim().to_string());
    }

    assert_eq!(tokens.join(" "), "four five six");
}

#[tokio::test]
async fn test_mock_max_tokens() {
    let mock = MockInference::new(vec!["one two three four five".into()]);

    let req = CompleteRequest {
        prompt: "test".into(),
        max_tokens: 3,
        temperature: 0.7,
        top_p: 0.9,
        stop: vec![],
        seed: None,
    };

    let res = mock.complete(req).await.unwrap();
    assert_eq!(res.text, "one two three");
    assert_eq!(res.finish_reason, ybos_inference::FinishReason::MaxTokens);
}
