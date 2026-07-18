use async_trait::async_trait;

use crate::domain::DomainResult;

#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub text_body: String,
    pub html_body: Option<String>,
}

#[async_trait]
pub trait Mailer: Send + Sync {
    fn is_configured(&self) -> bool;
    async fn send(&self, message: EmailMessage) -> DomainResult<()>;
}
