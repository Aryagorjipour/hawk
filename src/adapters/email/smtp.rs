use async_trait::async_trait;
use lettre::message::{Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use tracing::warn;

use crate::domain::{DomainError, DomainResult};
use crate::ports::{EmailMessage, Mailer};

pub struct SmtpMailer {
    transport: Option<AsyncSmtpTransport<Tokio1Executor>>,
    from: Option<Mailbox>,
}

impl SmtpMailer {
    pub fn from_config(smtp_url: Option<&str>, from: Option<&str>) -> DomainResult<Self> {
        let (transport, from_mb) = match (smtp_url, from) {
            (Some(url), Some(from_addr)) if !url.is_empty() => {
                let mailbox: Mailbox = from_addr
                    .parse()
                    .map_err(|e| DomainError::Email(format!("invalid SMTP_FROM: {e}")))?;
                let transport = build_transport(url)?;
                (Some(transport), Some(mailbox))
            }
            _ => {
                warn!("SMTP not fully configured; email delivery disabled");
                (None, None)
            }
        };
        Ok(Self {
            transport,
            from: from_mb,
        })
    }
}

fn build_transport(url: &str) -> DomainResult<AsyncSmtpTransport<Tokio1Executor>> {
    // Supports smtp://user:pass@host:port or smtp+tls / smtps
    if let Ok(transport) = AsyncSmtpTransport::<Tokio1Executor>::from_url(url) {
        return Ok(transport.build());
    }

    // Fallback: host:port with optional SMTP_USER/PASS already in URL form failed
    // Try plain host
    let builder = AsyncSmtpTransport::<Tokio1Executor>::relay(url)
        .map_err(|e| DomainError::Email(format!("smtp relay: {e}")))?;
    Ok(builder.build())
}

/// Allow constructing with explicit host credentials (tests / advanced).
pub fn smtp_with_creds(
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    from: &str,
) -> DomainResult<SmtpMailer> {
    let mailbox: Mailbox = from
        .parse()
        .map_err(|e| DomainError::Email(format!("invalid from: {e}")))?;
    let creds = Credentials::new(user.to_string(), pass.to_string());
    let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
        .map_err(|e| DomainError::Email(e.to_string()))?
        .port(port)
        .credentials(creds)
        .build();
    Ok(SmtpMailer {
        transport: Some(transport),
        from: Some(mailbox),
    })
}

#[async_trait]
impl Mailer for SmtpMailer {
    fn is_configured(&self) -> bool {
        self.transport.is_some() && self.from.is_some()
    }

    async fn send(&self, message: EmailMessage) -> DomainResult<()> {
        let transport = self
            .transport
            .as_ref()
            .ok_or_else(|| DomainError::Email("SMTP not configured".into()))?;
        let from = self
            .from
            .clone()
            .ok_or_else(|| DomainError::Email("SMTP_FROM not configured".into()))?;
        let to: Mailbox = message
            .to
            .parse()
            .map_err(|e| DomainError::Email(format!("invalid recipient: {e}")))?;

        let builder = Message::builder()
            .from(from)
            .to(to)
            .subject(message.subject);

        let email = if let Some(html) = message.html_body {
            builder
                .multipart(
                    MultiPart::alternative()
                        .singlepart(SinglePart::plain(message.text_body.clone()))
                        .singlepart(SinglePart::html(html)),
                )
                .map_err(|e| DomainError::Email(e.to_string()))?
        } else {
            builder
                .body(message.text_body)
                .map_err(|e| DomainError::Email(e.to_string()))?
        };

        transport
            .send(email)
            .await
            .map_err(|e| DomainError::Email(e.to_string()))?;
        Ok(())
    }
}

/// No-op mailer for tests / unconfigured mode with explicit type.
pub struct NullMailer;

#[async_trait]
impl Mailer for NullMailer {
    fn is_configured(&self) -> bool {
        false
    }

    async fn send(&self, _message: EmailMessage) -> DomainResult<()> {
        Err(DomainError::Email("mailer not configured".into()))
    }
}
