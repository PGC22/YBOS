use ybos_firewall::{MockJudge, Judge, JudgeContext, JudgeDecision, ContentType};

#[tokio::test]
async fn test_mock_allow_default() {
    let judge = MockJudge::new(vec![]);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let payload = b"hello world";
    let decision = judge.evaluate(payload, context).await.unwrap();
    assert_eq!(decision, JudgeDecision::Allow);
}

#[tokio::test]
async fn test_mock_block_on_pattern() {
    let judge = MockJudge::new(vec![
        ("secret".into(), JudgeDecision::Block { reason: "Contains 'secret'".into() })
    ]);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let payload = b"this has a secret in it";
    let decision = judge.evaluate(payload, context).await.unwrap();
    match decision {
        JudgeDecision::Block { reason } => assert_eq!(reason, "Contains 'secret'"),
        _ => panic!("Expected block"),
    }
}

#[tokio::test]
async fn test_mock_first_match_wins() {
    let judge = MockJudge::new(vec![
        ("first".into(), JudgeDecision::Block { reason: "first matched".into() }),
        ("second".into(), JudgeDecision::Block { reason: "second matched".into() }),
    ]);
    let context = JudgeContext {
        agent_name: "test_agent".into(),
        destination: "example.com".into(),
        purpose: None,
        content_type: ContentType::Text,
    };
    let payload = b"first and second";
    let decision = judge.evaluate(payload, context).await.unwrap();
    match decision {
        JudgeDecision::Block { reason } => assert_eq!(reason, "first matched"),
        _ => panic!("Expected block"),
    }
}
