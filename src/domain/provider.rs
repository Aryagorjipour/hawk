use serde::{Deserialize, Serialize};
use std::fmt;
use url::Url;

use super::error::{DomainError, DomainResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
    Gemini,
    Grok,
    OpenRouter,
    Custom,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Grok => "grok",
            Self::OpenRouter => "openrouter",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "openai" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            "grok" => Ok(Self::Grok),
            "openrouter" => Ok(Self::OpenRouter),
            "custom" => Ok(Self::Custom),
            other => Err(DomainError::Validation(format!(
                "unknown provider: {other}"
            ))),
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Gemini",
            Self::Grok => "Grok",
            Self::OpenRouter => "OpenRouter",
            Self::Custom => "Custom",
        }
    }

    pub fn default_base_url(self) -> Option<&'static str> {
        match self {
            Self::OpenAi => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com/v1"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta/openai"),
            Self::Grok => Some("https://api.x.ai/v1"),
            Self::OpenRouter => Some("https://openrouter.ai/api/v1"),
            Self::Custom => None,
        }
    }

    pub fn requires_custom_base_url(self) -> bool {
        matches!(self, Self::Custom)
    }

    pub fn all() -> &'static [ProviderKind] {
        &[
            Self::OpenAi,
            Self::Anthropic,
            Self::Gemini,
            Self::Grok,
            Self::OpenRouter,
            Self::Custom,
        ]
    }
}

impl fmt::Display for ProviderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelId(String);

impl ModelId {
    pub fn new(raw: impl Into<String>) -> DomainResult<Self> {
        let s = raw.into().trim().to_string();
        if s.is_empty() {
            return Err(DomainError::Validation("model id must not be empty".into()));
        }
        if s.len() > 256 {
            return Err(DomainError::Validation("model id too long".into()));
        }
        Ok(Self(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Validates OpenAI-compatible base URLs (must include API version path).
pub fn validate_base_url(raw: &str) -> DomainResult<Url> {
    let trimmed = raw.trim().trim_end_matches('/');
    let url = Url::parse(trimmed)
        .map_err(|e| DomainError::Validation(format!("invalid base URL: {e}")))?;

    if url.scheme() != "https" {
        return Err(DomainError::Validation("base URL must use https".into()));
    }

    if url.host_str().is_none() {
        return Err(DomainError::Validation(
            "base URL must include a host".into(),
        ));
    }

    let path = url.path();
    let has_version = path.split('/').filter(|s| !s.is_empty()).any(|seg| {
        let lower = seg.to_ascii_lowercase();
        let mut chars = lower.chars();
        chars.next() == Some('v') && chars.next().is_some_and(|c| c.is_ascii_digit())
    });

    if !has_version {
        return Err(DomainError::Validation(
            "base URL must include an API version path (e.g. /v1 or /v1beta)".into(),
        ));
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_openai_style_base_url() {
        assert!(validate_base_url("https://api.openai.com/v1").is_ok());
        assert!(validate_base_url("https://openrouter.ai/api/v1/").is_ok());
        assert!(validate_base_url("https://host.example/v1beta/openai").is_ok());
    }

    #[test]
    fn rejects_missing_version() {
        let err = validate_base_url("https://api.example.com").unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }

    #[test]
    fn rejects_http() {
        let err = validate_base_url("http://api.example.com/v1").unwrap_err();
        assert!(matches!(err, DomainError::Validation(_)));
    }
}
