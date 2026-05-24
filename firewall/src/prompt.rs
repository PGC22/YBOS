use crate::types::{JudgeContext, JudgePolicy};

pub fn build_prompt(payload: &[u8], context: &JudgeContext, policy: JudgePolicy) -> String {
    let policy_str = match policy {
        JudgePolicy::Strict => "Strict",
        JudgePolicy::Standard => "Standard",
        JudgePolicy::Permissive => "Permissive",
    };

    let content_type_str = match context.content_type {
        crate::types::ContentType::Text => "Text",
        crate::types::ContentType::Json => "Json",
        crate::types::ContentType::Other => "Other",
    };

    let purpose_str = context.purpose.as_deref().unwrap_or("(none provided)");

    let mut payload_str = String::from_utf8_lossy(payload).into_owned();
    if payload_str.len() > 4096 {
        // Find the largest index <= 4096 that is a char boundary
        let mut limit = 4096;
        while limit > 0 && !payload_str.is_char_boundary(limit) {
            limit -= 1;
        }
        payload_str.truncate(limit);
        payload_str.push_str("...[truncated]");
    }

    format!(
        "You are a privacy firewall judge for the YBOS personal AI OS.\n\
         Decide whether the following payload should be sent to a cloud LLM.\n\
         Policy: {}\n\
         Agent: {}\n\
         Destination: {}\n\
         Purpose: {}\n\
         Content type: {}\n\n\
         Payload (between BEGIN and END markers):\n\
         BEGIN_PAYLOAD\n\
         {}\n\
         END_PAYLOAD\n\n\
         Return a JSON object on one line with this exact schema:\n\
         {{\"decision\": \"allow\"}} for clean payloads,\n\
         {{\"decision\": \"redact\", \"redacted_payload\": \"<safe replacement>\", \"reasons\": [\"...\"]}} to strip PII/secrets,\n\
         {{\"decision\": \"block\", \"reason\": \"...\"}} for high-risk content,\n\
         {{\"decision\": \"ask_user\", \"prompt\": \"...\"}} for ambiguous cases (only under Strict/Standard policy).\n\n\
         Apply the policy:\n\
         - Strict: prefer block > redact > ask_user > allow.\n\
         - Standard: prefer redact for PII (emails, phones, IDs); ask_user on doubt; allow generic.\n\
         - Permissive: redact only secrets/API keys; never block; never ask.\n\n\
         Return only the JSON object, no commentary.",
        policy_str,
        context.agent_name,
        context.destination,
        purpose_str,
        content_type_str,
        payload_str
    )
}
