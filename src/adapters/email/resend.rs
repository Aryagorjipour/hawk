use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::time::Duration;
use tracing::debug;

use crate::domain::{DomainError, DomainResult};
use crate::ports::{EmailMessage, Mailer};

pub struct ResendMailer {
    http: Client,
    api_key: String,
    from: String,
}

impl ResendMailer {
    pub fn new(api_key: impl Into<String>, from: impl Into<String>) -> DomainResult<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| DomainError::Email(e.to_string()))?;
        Ok(Self {
            http,
            api_key: api_key.into(),
            from: from.into(),
        })
    }
}

#[derive(Serialize)]
struct ResendPayload<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    html: Option<&'a str>,
}

#[async_trait]
impl Mailer for ResendMailer {
    fn is_configured(&self) -> bool {
        !self.api_key.is_empty() && !self.from.is_empty()
    }

    async fn send(&self, message: EmailMessage) -> DomainResult<()> {
        let payload = ResendPayload {
            from: &self.from,
            to: vec![message.to.as_str()],
            subject: &message.subject,
            text: &message.text_body,
            html: message.html_body.as_deref(),
        };

        let res = self
            .http
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| DomainError::Email(format!("resend network: {e}")))?;

        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        if !status.is_success() {
            debug!(%status, %body, "resend_error");
            let hint = crate::domain::extract_provider_hint(&body)
                .unwrap_or_else(|| format!("HTTP {status}"));
            return Err(DomainError::Email(format!("resend failed: {hint}")));
        }
        Ok(())
    }
}
