use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::credits::CreditBalance;
use super::error::{DomainError, DomainResult};
use super::ids::{TelegramUserId, UserId};
use super::locale::Locale;
use super::provider::{validate_base_url, ModelId, ProviderKind};
use super::secret::EncryptedBlob;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStatus {
    NotStarted,
    InProgress,
    Completed,
}

impl OnboardingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not_started",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "not_started" => Ok(Self::NotStarted),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            other => Err(DomainError::Parse(format!(
                "unknown onboarding status: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiConfig {
    pub provider: ProviderKind,
    pub base_url: String,
    pub api_key: EncryptedBlob,
    pub model_id: ModelId,
    pub connection_verified_at: Option<DateTime<Utc>>,
}

impl AiConfig {
    pub fn is_verified(&self) -> bool {
        self.connection_verified_at.is_some()
    }

    pub fn mark_verified(&mut self, at: DateTime<Utc>) {
        self.connection_verified_at = Some(at);
    }

    pub fn clear_verification(&mut self) {
        self.connection_verified_at = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub telegram_user_id: TelegramUserId,
    pub display_name: String,
    pub email: Option<String>,
    pub timezone: String,
    pub locale: Locale,
    pub onboarding_status: OnboardingStatus,
    pub ai_config: Option<AiConfig>,
    pub credits: CreditBalance,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn new(
        telegram_user_id: TelegramUserId,
        display_name: String,
        locale: Locale,
        now: DateTime<Utc>,
    ) -> DomainResult<Self> {
        let name = display_name.trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "display name must not be empty".into(),
            ));
        }
        Ok(Self {
            id: UserId::new(),
            telegram_user_id,
            display_name: name,
            email: None,
            timezone: "UTC".into(),
            locale,
            onboarding_status: OnboardingStatus::InProgress,
            ai_config: None,
            credits: CreditBalance::default(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn set_display_name(
        &mut self,
        name: impl Into<String>,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        let name = name.into().trim().to_string();
        if name.is_empty() {
            return Err(DomainError::Validation(
                "display name must not be empty".into(),
            ));
        }
        if name.len() > 64 {
            return Err(DomainError::Validation(
                "display name too long (max 64)".into(),
            ));
        }
        self.display_name = name;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_email(&mut self, email: Option<String>, now: DateTime<Utc>) -> DomainResult<()> {
        if let Some(ref e) = email {
            let e = e.trim();
            if e.is_empty() {
                self.email = None;
            } else if !is_plausible_email(e) {
                return Err(DomainError::Validation("invalid email address".into()));
            } else {
                self.email = Some(e.to_string());
            }
        } else {
            self.email = None;
        }
        self.updated_at = now;
        Ok(())
    }

    pub fn set_timezone(&mut self, tz: impl Into<String>, now: DateTime<Utc>) -> DomainResult<()> {
        let tz = tz.into();
        if tz.parse::<chrono_tz::Tz>().is_err() && tz != "UTC" {
            return Err(DomainError::Validation(format!(
                "unknown IANA timezone: {tz}"
            )));
        }
        self.timezone = if tz == "UTC" { "UTC".into() } else { tz };
        self.updated_at = now;
        Ok(())
    }

    pub fn set_locale(&mut self, locale: Locale, now: DateTime<Utc>) {
        self.locale = locale;
        self.updated_at = now;
    }

    pub fn set_ai_partial(
        &mut self,
        provider: ProviderKind,
        base_url: Option<String>,
        api_key: EncryptedBlob,
        model_id: ModelId,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        let base = match provider {
            ProviderKind::Custom => {
                let raw = base_url.ok_or_else(|| {
                    DomainError::Validation("custom provider requires base URL".into())
                })?;
                validate_base_url(&raw)?.to_string()
            }
            other => {
                if let Some(raw) = base_url {
                    validate_base_url(&raw)?.to_string()
                } else {
                    other
                        .default_base_url()
                        .ok_or_else(|| DomainError::Validation("missing default base URL".into()))?
                        .to_string()
                }
            }
        };

        self.ai_config = Some(AiConfig {
            provider,
            base_url: base.trim_end_matches('/').to_string(),
            api_key,
            model_id,
            connection_verified_at: None,
        });
        self.updated_at = now;
        Ok(())
    }

    pub fn mark_ai_verified(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        let cfg = self.ai_config.as_mut().ok_or(DomainError::AiNotVerified)?;
        cfg.mark_verified(now);
        self.onboarding_status = OnboardingStatus::Completed;
        self.updated_at = now;
        Ok(())
    }

    pub fn ensure_ready_to_crawl(&self) -> DomainResult<()> {
        if self.onboarding_status != OnboardingStatus::Completed {
            return Err(DomainError::OnboardingIncomplete);
        }
        match &self.ai_config {
            Some(c) if c.is_verified() => Ok(()),
            _ => Err(DomainError::AiNotVerified),
        }
    }
}

fn is_plausible_email(s: &str) -> bool {
    let parts: Vec<&str> = s.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && s.len() <= 254
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap()
    }

    #[test]
    fn creates_in_progress_user() {
        let u = User::new(TelegramUserId::new(1), "Arya".into(), Locale::En, now()).unwrap();
        assert_eq!(u.onboarding_status, OnboardingStatus::InProgress);
        assert!(u.ai_config.is_none());
    }

    #[test]
    fn rejects_bad_email() {
        let mut u = User::new(TelegramUserId::new(1), "Arya".into(), Locale::En, now()).unwrap();
        assert!(u.set_email(Some("not-an-email".into()), now()).is_err());
    }
}
