use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use crate::bootstrap::Config;
use crate::domain::{DomainError, DomainResult};
use crate::ports::{EmailMessage, Mailer};

use super::resend::ResendMailer;
use super::smtp::{NullMailer, SmtpMailer};

/// Tries Resend first when configured, then SMTP.
pub struct CompositeMailer {
    resend: Option<ResendMailer>,
    smtp: Option<SmtpMailer>,
}

impl CompositeMailer {
    pub fn from_config(config: &Config) -> DomainResult<Self> {
        info!(diag = %config.email_diag(), "email_config");

        let resend = match (
            config.resend_api_key.as_deref(),
            config.resend_from.as_deref(),
        ) {
            (Some(key), Some(from)) if !key.is_empty() && !from.is_empty() => {
                info!(from_len = from.len(), "email: Resend primary configured");
                Some(ResendMailer::new(key, from)?)
            }
            (Some(_), None) => {
                warn!(
                    "RESEND_API_KEY set but RESEND_FROM/EMAIL_FROM missing or empty — Resend disabled. \
                     Use a quoted value in .env, e.g. RESEND_FROM=\"Smart Hawk <you@domain.com>\" \
                     or bare: RESEND_FROM=you@domain.com"
                );
                None
            }
            (None, Some(_)) => {
                warn!("RESEND_FROM set but RESEND_API_KEY missing — Resend disabled");
                None
            }
            _ => None,
        };

        let smtp = match (config.smtp_url.as_deref(), config.smtp_from.as_deref()) {
            (Some(url), Some(from)) if !url.is_empty() => {
                info!("email: SMTP fallback configured");
                Some(SmtpMailer::from_config(Some(url), Some(from))?)
            }
            _ => {
                if resend.is_none() {
                    warn!(
                        "no Resend or SMTP configured — email delivery disabled ({})",
                        config.email_diag()
                    );
                }
                None
            }
        };

        Ok(Self { resend, smtp })
    }
}

#[async_trait]
impl Mailer for CompositeMailer {
    fn is_configured(&self) -> bool {
        self.resend
            .as_ref()
            .map(|r| r.is_configured())
            .unwrap_or(false)
            || self
                .smtp
                .as_ref()
                .map(|s| s.is_configured())
                .unwrap_or(false)
    }

    async fn send(&self, message: EmailMessage) -> DomainResult<()> {
        if let Some(resend) = &self.resend {
            if resend.is_configured() {
                match resend.send(message.clone()).await {
                    Ok(()) => return Ok(()),
                    Err(e) => {
                        warn!(error = %e, "resend_failed_trying_smtp_fallback");
                        if self.smtp.as_ref().is_none_or(|s| !s.is_configured()) {
                            return Err(e);
                        }
                    }
                }
            }
        }

        if let Some(smtp) = &self.smtp {
            if smtp.is_configured() {
                return smtp.send(message).await;
            }
        }

        Err(DomainError::Email(
            "no email backend configured (set RESEND_API_KEY + RESEND_FROM, or SMTP_URL + SMTP_FROM)"
                .into(),
        ))
    }
}

pub fn build_mailer(config: &Config) -> DomainResult<Arc<dyn Mailer>> {
    let composite = CompositeMailer::from_config(config)?;
    if composite.is_configured() {
        Ok(Arc::new(composite))
    } else {
        Ok(Arc::new(NullMailer))
    }
}
