use std::sync::Arc;
use ybos_firewall::{LlmJudge, Judge, JudgeContext, JudgeDecision, ContentType, JudgePolicy};
use ybos_inference::MockInference;

#[tokio::test]
async fn test_integration_standard_policy_block() {
    let mock_res = "Some thinking... {\"decision\":\"block\",\"reason\":\"API key detected\"} more text";
    let mock_inference = Arc::new(MockInference::new(vec![mock_res.into()]));
    let judge = LlmJudge::new(mock_inference, JudgePolicy::Standard);

    let context = JudgeContext {
        agent_name: "integrator".into(),
        destination: "cloud.api".into(),
        purpose: Some("testing integration".into()),
        content_type: ContentType::Text,
    };

    let payload = b"my key is sk-1234567890abcdef";
    let decision = judge.evaluate(payload, context).await.unwrap();

    match decision {
        JudgeDecision::Block { reason } => assert_eq!(reason, "API key detected"),
        _ => panic!("Expected block"),
    }
}
