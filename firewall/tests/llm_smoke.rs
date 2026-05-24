use std::sync::Arc;
use ybos_firewall::{LlmJudge, Judge, JudgeContext, JudgeDecision, ContentType, JudgePolicy};
use ybos_inference::MockInference;

#[tokio::test]
async fn test_llm_allow() {
    let mock_inference = Arc::new(MockInference::new(vec!["{\"decision\":\"allow\"}".into()]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    assert_eq!(decision, JudgeDecision::Allow);
}

#[tokio::test]
async fn test_llm_block() {
    let mock_inference = Arc::new(MockInference::new(vec![
        "{\"decision\":\"block\",\"reason\":\"contains password\"}".into()
    ]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    match decision {
        JudgeDecision::Block { reason } => assert_eq!(reason, "contains password"),
        _ => panic!("Expected block"),
    }
}

#[tokio::test]
async fn test_llm_redact() {
    let mock_inference = Arc::new(MockInference::new(vec![
        "{\"decision\":\"redact\",\"redacted_payload\":\"hi <REDACTED>\",\"reasons\":[\"email\"]}".into()
    ]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    match decision {
        JudgeDecision::Redact { redacted_payload, reasons } => {
            assert_eq!(redacted_payload, b"hi <REDACTED>");
            assert_eq!(reasons, vec!["email".to_string()]);
        }
        _ => panic!("Expected redact"),
    }
}

#[tokio::test]
async fn test_llm_ask_user() {
    let mock_inference = Arc::new(MockInference::new(vec![
        "{\"decision\":\"ask_user\",\"prompt\":\"OK to send your address?\"}".into()
    ]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    match decision {
        JudgeDecision::AskUser { prompt } => assert_eq!(prompt, "OK to send your address?"),
        _ => panic!("Expected ask_user"),
    }
}

#[tokio::test]
async fn test_llm_parse_failure_fails_closed() {
    let mock_inference = Arc::new(MockInference::new(vec!["<not json>".into()]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    match decision {
        JudgeDecision::Block { reason } => assert!(reason.contains("unparseable")),
        _ => panic!("Expected block on parse failure"),
    }
}

#[tokio::test]
async fn test_llm_parse_stray_close_fails_closed() {
    let mock_inference = Arc::new(MockInference::new(vec![
        "junk } {\"decision\":\"allow\"}".into()
    ]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let decision = judge.evaluate(b"hello", context).await.unwrap();
    match decision {
        JudgeDecision::Block { reason } => assert!(reason.contains("unparseable")),
        _ => panic!("Expected block on stray close brace"),
    }
}
