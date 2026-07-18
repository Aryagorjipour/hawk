use std::sync::Arc;

use crate::domain::AiConfig;
use crate::domain::{ApiKey, DomainResult, ProviderKind};
use crate::ports::{LlmClient, LlmClientConfig, SecretBox};

use super::openai_compat::OpenAiCompatClient;

pub fn build_llm_client(
    ai: &AiConfig,
    secret_box: &dyn SecretBox,
) -> DomainResult<Arc<dyn LlmClient>> {
    let key: ApiKey = secret_box.decrypt(&ai.api_key)?;
    let cfg = LlmClientConfig {
        base_url: ai.base_url.clone(),
        api_key: key.expose().to_string(),
        provider_label: ai.provider.display_name().to_string(),
    };
    let client = OpenAiCompatClient::new(cfg)?;
    Ok(Arc::new(client))
}

pub fn build_llm_client_raw(
    provider: ProviderKind,
    base_url: &str,
    api_key: &ApiKey,
) -> DomainResult<Arc<dyn LlmClient>> {
    let cfg = LlmClientConfig {
        base_url: base_url.to_string(),
        api_key: api_key.expose().to_string(),
        provider_label: provider.display_name().to_string(),
    };
    Ok(Arc::new(OpenAiCompatClient::new(cfg)?))
}
