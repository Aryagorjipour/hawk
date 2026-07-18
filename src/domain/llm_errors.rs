use super::error::LlmErrorKind;

/// Classify provider HTTP failures from status + body (quota is not auth).
pub fn classify_llm_http(status: u16, body: &str) -> LlmErrorKind {
    let lower = body.to_ascii_lowercase();

    if looks_like_quota(&lower, status) {
        return LlmErrorKind::InsufficientQuota;
    }
    if looks_like_rate_limit(&lower, status) {
        return LlmErrorKind::RateLimit;
    }
    if looks_like_model(&lower, status) {
        return LlmErrorKind::Model;
    }
    if looks_like_auth(&lower, status) {
        return LlmErrorKind::Auth;
    }

    match status {
        401 => LlmErrorKind::Auth,
        402 => LlmErrorKind::InsufficientQuota,
        403 => LlmErrorKind::Auth,
        429 => LlmErrorKind::RateLimit,
        408 | 502 | 503 | 504 => LlmErrorKind::Network,
        s if s >= 500 => LlmErrorKind::Network,
        404 | 400 => LlmErrorKind::Unknown,
        _ => LlmErrorKind::Unknown,
    }
}

fn looks_like_quota(lower: &str, status: u16) -> bool {
    const MARKERS: &[&str] = &[
        "insufficient_user_quota",
        "insufficient_quota",
        "pre-consume quota",
        "pre_consume_quota",
        "remaining user quota",
        "required pre-consume quota",
        "quota exceeded",
        "exceeded your current quota",
        "you exceeded your current quota",
        "billing_hard_limit",
        "billing hard limit",
        "credit balance is too low",
        "insufficient credits",
        "out of credits",
        "not enough credits",
        "payment required",
        "spend limit",
        "budget exceeded",
        "account_deactivated", // sometimes balance related on gateways
        "gap_api_error",
    ];
    if MARKERS.iter().any(|m| lower.contains(m)) {
        return true;
    }
    // bare "quota" with money amounts often means balance
    if lower.contains("quota")
        && (lower.contains('$')
            || lower.contains("usd")
            || lower.contains("balance")
            || lower.contains("credit")
            || status == 402
            || status == 403)
    {
        return true;
    }
    status == 402
}

fn looks_like_rate_limit(lower: &str, status: u16) -> bool {
    status == 429
        || lower.contains("rate_limit")
        || lower.contains("rate limit")
        || lower.contains("too many requests")
        || lower.contains("tokens per min")
}

fn looks_like_model(lower: &str, status: u16) -> bool {
    (status == 404 || status == 400)
        && (lower.contains("model")
            || lower.contains("does not exist")
            || lower.contains("not found")
            || lower.contains("invalid model"))
}

fn looks_like_auth(lower: &str, status: u16) -> bool {
    if status != 401 && status != 403 {
        return false;
    }
    // Prefer quota over auth when both could match 403
    if looks_like_quota(lower, status) {
        return false;
    }
    lower.contains("invalid_api_key")
        || lower.contains("incorrect api key")
        || lower.contains("invalid authentication")
        || lower.contains("authentication")
        || lower.contains("unauthorized")
        || lower.contains("permission")
        || lower.contains("forbidden")
        || lower.contains("access denied")
        || lower.contains("api key")
        || status == 401
}

/// Pull a short provider message from JSON bodies without dumping the whole blob.
pub fn extract_provider_hint(body: &str) -> Option<String> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(msg) = v
            .pointer("/error/message")
            .or_else(|| v.pointer("/message"))
            .or_else(|| v.pointer("/error"))
            .and_then(|x| x.as_str())
        {
            return Some(sanitize_hint(msg));
        }
        if let Some(code) = v
            .pointer("/error/code")
            .or_else(|| v.pointer("/code"))
            .and_then(|x| x.as_str())
        {
            return Some(sanitize_hint(code));
        }
    }

    // Non-JSON: first line, truncated
    let line = trimmed.lines().next().unwrap_or(trimmed);
    Some(sanitize_hint(line))
}

fn sanitize_hint(s: &str) -> String {
    let one_line: String = s.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    let t = one_line.trim();
    if t.chars().count() > 160 {
        format!("{}…", t.chars().take(160).collect::<String>())
    } else {
        t.to_string()
    }
}

/// Stable code stored on crawl jobs for i18n at delivery time.
pub fn llm_error_code(kind: LlmErrorKind) -> &'static str {
    match kind {
        LlmErrorKind::Auth => "llm_auth",
        LlmErrorKind::InsufficientQuota => "llm_quota",
        LlmErrorKind::RateLimit => "llm_rate",
        LlmErrorKind::Model => "llm_model",
        LlmErrorKind::Network => "llm_network",
        LlmErrorKind::InvalidResponse => "llm_bad_response",
        LlmErrorKind::Unknown => "llm_unknown",
    }
}

/// English user-facing line (fallback when i18n not available).
pub fn llm_user_message_en(kind: LlmErrorKind) -> &'static str {
    match kind {
        LlmErrorKind::Auth => {
            "Your AI API key was rejected. Check the key in Settings → AI provider."
        }
        LlmErrorKind::InsufficientQuota => {
            "Your AI provider has no (or not enough) credit left for this request. Top up balance at the provider, then try again."
        }
        LlmErrorKind::RateLimit => {
            "Your AI provider is rate-limiting you. Wait a bit, then try again."
        }
        LlmErrorKind::Model => {
            "That model is unavailable or invalid. Pick another model in Settings → AI provider."
        }
        LlmErrorKind::Network => {
            "Could not reach your AI provider (network/timeout). Try again in a moment."
        }
        LlmErrorKind::InvalidResponse => {
            "Your AI provider returned a response we could not understand. Try another model or provider."
        }
        LlmErrorKind::Unknown => {
            "Your AI provider returned an error. Check Settings → AI provider, balance, and model."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_gap_quota_403_as_quota_not_auth() {
        let body = r#"{"error":{"message":"pre-consume quota failed, remaining user quota: $0.029750, required pre-consume quota: $0.149124 (request id: x)","type":"gap_api_error","code":"insufficient_user_quota"}}"#;
        assert_eq!(
            classify_llm_http(403, body),
            LlmErrorKind::InsufficientQuota
        );
    }

    #[test]
    fn classifies_openai_quota() {
        let body = r#"{"error":{"message":"You exceeded your current quota","type":"insufficient_quota","code":"insufficient_quota"}}"#;
        assert_eq!(
            classify_llm_http(429, body),
            LlmErrorKind::InsufficientQuota
        );
    }

    #[test]
    fn classifies_invalid_key() {
        let body = r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error","code":"invalid_api_key"}}"#;
        assert_eq!(classify_llm_http(401, body), LlmErrorKind::Auth);
    }

    #[test]
    fn extracts_message() {
        let body =
            r#"{"error":{"message":"pre-consume quota failed","code":"insufficient_user_quota"}}"#;
        let hint = extract_provider_hint(body).unwrap();
        assert!(hint.contains("pre-consume"));
        assert!(!hint.contains('{'));
    }
}
