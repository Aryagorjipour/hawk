use async_trait::async_trait;

use crate::domain::{DomainResult, LlmErrorKind, ModelId, StructuredCrawlResult};

#[derive(Debug, Clone)]
pub struct LlmModelInfo {
    pub id: String,
    pub owned_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PageContext {
    pub url: String,
    pub title: Option<String>,
    pub text: String,
    pub fetch_mode: String,
}

#[derive(Debug, Clone)]
pub struct ExtractRequest {
    pub model: ModelId,
    pub user_prompt: String,
    pub pages: Vec<PageContext>,
    pub response_language_hint: Option<String>,
}

#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn list_models(&self) -> DomainResult<Vec<LlmModelInfo>>;
    async fn probe_connection(&self, model: &ModelId) -> DomainResult<()>;
    async fn extract(&self, req: ExtractRequest) -> DomainResult<StructuredCrawlResult>;
    async fn sanity_check_title(
        &self,
        model: &ModelId,
        expected_sample: &str,
        page_excerpt: &str,
    ) -> DomainResult<String>;
}

#[derive(Debug, Clone)]
pub struct LlmClientConfig {
    pub base_url: String,
    pub api_key: String,
    pub provider_label: String,
}

/// Classifies HTTP-ish failures for onboarding error routing.
/// Prefer [`crate::domain::classify_llm_http`] — this wraps it for port consumers.
pub fn classify_http_status(status: u16, body: &str) -> LlmErrorKind {
    crate::domain::classify_llm_http(status, body)
}
