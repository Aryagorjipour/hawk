use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::domain::{
    parse_structured_result, DomainError, DomainResult, LlmErrorKind, ModelId,
    StructuredCrawlResult, MAX_PAGE_CONTEXT_CHARS,
};
use crate::ports::{
    classify_http_status, ExtractRequest, LlmClient, LlmClientConfig, LlmModelInfo, PageContext,
};

pub struct OpenAiCompatClient {
    http: Client,
    base_url: String,
    api_key: String,
    provider_label: String,
}

impl OpenAiCompatClient {
    pub fn new(cfg: LlmClientConfig) -> DomainResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(90))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::Network,
                detail: e.to_string(),
            })?;
        Ok(Self {
            http,
            base_url: cfg.base_url.trim_end_matches('/').to_string(),
            api_key: cfg.api_key,
            provider_label: cfg.provider_label,
        })
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.api_key)
    }

    fn map_error(&self, status: u16, body: &str) -> DomainError {
        let kind = classify_http_status(status, body);
        tracing::debug!(
            provider = %self.provider_label,
            %status,
            kind = %kind,
            body = %truncate(body, 500),
            "llm_http_error"
        );
        let hint =
            crate::domain::extract_provider_hint(body).unwrap_or_else(|| format!("HTTP {status}"));
        // Keep detail short for logs/operators — never store full JSON in user-visible paths.
        let detail = format!("{} · {hint}", self.provider_label);
        DomainError::Llm { kind, detail }
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatClient {
    async fn list_models(&self) -> DomainResult<Vec<LlmModelInfo>> {
        let url = format!("{}/models", self.base_url);
        let res = self
            .http
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::Network,
                detail: e.to_string(),
            })?;
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(self.map_error(status, &body));
        }
        let parsed: ModelsResponse = serde_json::from_str(&body).map_err(|e| DomainError::Llm {
            kind: LlmErrorKind::InvalidResponse,
            detail: e.to_string(),
        })?;
        let mut models: Vec<LlmModelInfo> = parsed
            .data
            .into_iter()
            .map(|m| LlmModelInfo {
                id: m.id,
                owned_by: m.owned_by,
            })
            .collect();
        models.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(models)
    }

    async fn probe_connection(&self, model: &ModelId) -> DomainResult<()> {
        let url = format!("{}/chat/completions", self.base_url);
        let payload = ChatCompletionRequest {
            model: model.as_str().to_string(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: "Reply with exactly: pong".into(),
            }],
            temperature: Some(0.0),
            max_tokens: Some(16),
        };
        let res = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::Network,
                detail: e.to_string(),
            })?;
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(self.map_error(status, &body));
        }
        let _parsed: ChatCompletionResponse =
            serde_json::from_str(&body).map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::InvalidResponse,
                detail: e.to_string(),
            })?;
        Ok(())
    }

    async fn extract(&self, req: ExtractRequest) -> DomainResult<StructuredCrawlResult> {
        let system = SYSTEM_EXTRACT.to_string();
        let user = build_extract_user_message(&req);
        let raw = self.chat_raw(&req.model, system, user, 0.1, 4096).await?;
        parse_structured_result(&raw)
    }

    async fn sanity_check_title(
        &self,
        model: &ModelId,
        expected_sample: &str,
        page_excerpt: &str,
    ) -> DomainResult<String> {
        let system = "You verify page content. Reply with one short line only.".into();
        let user = format!(
            "Given this page excerpt, what is the page title or a distinctive phrase you see?\n\
             Expected fingerprint hint: {expected_sample}\n\
             Excerpt:\n{}",
            truncate(page_excerpt, 1500)
        );
        self.chat_raw(model, system, user, 0.0, 64).await
    }
}

impl OpenAiCompatClient {
    async fn chat_raw(
        &self,
        model: &ModelId,
        system: String,
        user: String,
        temperature: f32,
        max_tokens: u32,
    ) -> DomainResult<String> {
        let url = format!("{}/chat/completions", self.base_url);
        let payload = ChatCompletionRequest {
            model: model.as_str().to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".into(),
                    content: system,
                },
                ChatMessage {
                    role: "user".into(),
                    content: user,
                },
            ],
            temperature: Some(temperature),
            max_tokens: Some(max_tokens),
        };
        let res = self
            .http
            .post(&url)
            .header("Authorization", self.auth_header())
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::Network,
                detail: e.to_string(),
            })?;
        let status = res.status().as_u16();
        let body = res.text().await.unwrap_or_default();
        if status >= 400 {
            return Err(self.map_error(status, &body));
        }
        let parsed: ChatCompletionResponse =
            serde_json::from_str(&body).map_err(|e| DomainError::Llm {
                kind: LlmErrorKind::InvalidResponse,
                detail: e.to_string(),
            })?;
        parsed
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| DomainError::Llm {
                kind: LlmErrorKind::InvalidResponse,
                detail: "empty choices from LLM".into(),
            })
    }
}

const SYSTEM_EXTRACT: &str = r#"You are Smart Hawk's extraction engine.
You receive fetched page content from a crawler. Use ONLY that content.
Respond with a single JSON object (no markdown fences unless necessary) matching:
{
  "status": "ok" | "partial" | "unable",
  "language": "language of the user request",
  "title": "string",
  "summary": "string",
  "items": [{"label":"string","value":"string","evidence":"optional"}],
  "sources": [{"url":"string","note":"string"}],
  "follow_up_urls": ["https://..."],
  "unable_reason": "only if status=unable"
}
Rules:
- Match the language of the user request for summary and item values.
- If content cannot answer the request, status=unable with a clear unable_reason.
- Propose at most 3 follow_up_urls only if they are clearly needed and present on the page.
- Never invent facts not supported by the provided content.
- Do not apologize; be precise."#;

fn build_extract_user_message(req: &ExtractRequest) -> String {
    let mut out = String::new();
    out.push_str("=== USER REQUEST (answer this) ===\n");
    out.push_str(&req.user_prompt);
    out.push_str("\n=== END USER REQUEST ===\n\n");
    if let Some(lang) = &req.response_language_hint {
        out.push_str(&format!("Preferred response language hint: {lang}\n\n"));
    }
    for (i, page) in req.pages.iter().enumerate() {
        out.push_str(&format!(
            "--- PAGE {} | {} | mode={} ---\n",
            i + 1,
            page.url,
            page.fetch_mode
        ));
        if let Some(t) = &page.title {
            out.push_str(&format!("Title: {t}\n"));
        }
        let text = if page.text.chars().count() > MAX_PAGE_CONTEXT_CHARS {
            let mut t: String = page.text.chars().take(MAX_PAGE_CONTEXT_CHARS).collect();
            t.push_str("\n…[truncated]");
            t
        } else {
            page.text.clone()
        };
        out.push_str(&text);
        out.push_str("\n\n");
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelObj>,
}

#[derive(Debug, Deserialize)]
struct ModelObj {
    id: String,
    owned_by: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

// silence unused import of PageContext in some builds
#[allow(dead_code)]
fn _touch_page(p: &PageContext) {
    let _ = &p.url;
}
