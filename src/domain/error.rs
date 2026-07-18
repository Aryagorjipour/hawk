use thiserror::Error;

use super::llm_errors::{llm_error_code, llm_user_message_en};

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("user not found")]
    UserNotFound,

    #[error("onboarding incomplete")]
    OnboardingIncomplete,

    #[error("AI connection not verified")]
    AiNotVerified,

    #[error("quota exceeded: {0}")]
    QuotaExceeded(String),

    #[error("schedule slot limit reached")]
    ScheduleSlotLimit,

    #[error("crawl already running for this user")]
    CrawlAlreadyRunning,

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("SSRF blocked: {0}")]
    SsrfBlocked(String),

    #[error("fetch failed: {0}")]
    FetchFailed(String),

    /// LLM provider failure. Prefer [`Self::user_message`] for chat UX — never dump raw JSON.
    #[error("{}", .kind.user_facing())]
    Llm {
        kind: LlmErrorKind,
        /// Short technical hint for logs / operators (not raw response bodies).
        detail: String,
    },

    #[error("parse error: {0}")]
    Parse(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("persistence error: {0}")]
    Persistence(String),

    #[error("email error: {0}")]
    Email(String),

    #[error("payment error: {0}")]
    Payment(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmErrorKind {
    Auth,
    /// Provider balance / pre-consume quota / billing — not a bad API key.
    InsufficientQuota,
    RateLimit,
    Model,
    Network,
    InvalidResponse,
    Unknown,
}

impl LlmErrorKind {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::InsufficientQuota => "insufficient_quota",
            Self::RateLimit => "rate_limit",
            Self::Model => "model",
            Self::Network => "network",
            Self::InvalidResponse => "invalid_response",
            Self::Unknown => "unknown",
        }
    }

    pub fn user_facing(self) -> &'static str {
        llm_user_message_en(self)
    }
}

impl std::fmt::Display for LlmErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_code())
    }
}

impl DomainError {
    /// Stable code for crawl_jobs.error_kind / i18n routing.
    pub fn error_code(&self) -> &'static str {
        match self {
            DomainError::Llm { kind, .. } => llm_error_code(*kind),
            DomainError::SsrfBlocked(_) => "ssrf",
            DomainError::InvalidUrl(_) => "invalid_url",
            DomainError::Validation(_) => "validation",
            DomainError::QuotaExceeded(_) => "bot_quota",
            DomainError::CrawlAlreadyRunning => "crawl_busy",
            DomainError::OnboardingIncomplete | DomainError::AiNotVerified => "need_onboarding",
            DomainError::ScheduleSlotLimit => "schedule_slots",
            DomainError::FetchFailed(_) => "fetch",
            DomainError::Parse(_) => "parse",
            DomainError::Email(_) => "email",
            _ => "error",
        }
    }

    /// i18n key for user-facing text when available.
    pub fn i18n_key(&self) -> Option<&'static str> {
        match self {
            DomainError::Llm {
                kind: LlmErrorKind::Auth,
                ..
            } => Some("error-llm-auth"),
            DomainError::Llm {
                kind: LlmErrorKind::InsufficientQuota,
                ..
            } => Some("error-llm-quota"),
            DomainError::Llm {
                kind: LlmErrorKind::RateLimit,
                ..
            } => Some("error-llm-rate"),
            DomainError::Llm {
                kind: LlmErrorKind::Model,
                ..
            } => Some("error-llm-model"),
            DomainError::Llm {
                kind: LlmErrorKind::Network,
                ..
            } => Some("error-llm-network"),
            DomainError::Llm {
                kind: LlmErrorKind::InvalidResponse,
                ..
            } => Some("error-llm-bad-response"),
            DomainError::Llm {
                kind: LlmErrorKind::Unknown,
                ..
            } => Some("error-llm-unknown"),
            DomainError::SsrfBlocked(_) => Some("error-ssrf"),
            DomainError::QuotaExceeded(_) => Some("crawl-quota"),
            DomainError::CrawlAlreadyRunning => Some("crawl-busy"),
            DomainError::OnboardingIncomplete | DomainError::AiNotVerified => {
                Some("crawl-need-onboarding")
            }
            DomainError::ScheduleSlotLimit => Some("schedule-slot-full"),
            _ => None,
        }
    }

    /// Friendly English message for chat / history (never raw provider JSON).
    pub fn user_message(&self) -> String {
        match self {
            DomainError::Llm { kind, .. } => llm_user_message_en(*kind).to_string(),
            DomainError::FetchFailed(d) => format!("Could not fetch the page: {d}"),
            DomainError::InvalidUrl(d) => format!("That URL is not usable: {d}"),
            DomainError::SsrfBlocked(_) => {
                "That address is off-limits (private/local network).".into()
            }
            DomainError::Validation(d) => d.clone(),
            DomainError::QuotaExceeded(_) => {
                "You're out of free crawls and bonus credits. Grab a Stars pack in About.".into()
            }
            DomainError::CrawlAlreadyRunning => "One hunt at a time, captain.".into(),
            DomainError::OnboardingIncomplete | DomainError::AiNotVerified => {
                "Finish onboarding first — I need your AI setup.".into()
            }
            DomainError::ScheduleSlotLimit => {
                "No free schedule slots. Buy a pack or deactivate one.".into()
            }
            other => other.to_string_fallback(),
        }
    }

    fn to_string_fallback(&self) -> String {
        // Avoid infinite recursion on Llm Display
        match self {
            DomainError::Llm { kind, detail } => {
                format!("{} ({})", llm_user_message_en(*kind), detail)
            }
            DomainError::Validation(s)
            | DomainError::QuotaExceeded(s)
            | DomainError::InvalidUrl(s)
            | DomainError::SsrfBlocked(s)
            | DomainError::FetchFailed(s)
            | DomainError::Parse(s)
            | DomainError::NotFound(s)
            | DomainError::Conflict(s)
            | DomainError::Crypto(s)
            | DomainError::Persistence(s)
            | DomainError::Email(s)
            | DomainError::Payment(s)
            | DomainError::Internal(s) => s.clone(),
            DomainError::UserNotFound => "user not found".into(),
            DomainError::OnboardingIncomplete => "onboarding incomplete".into(),
            DomainError::AiNotVerified => "AI not verified".into(),
            DomainError::ScheduleSlotLimit => "schedule slot limit".into(),
            DomainError::CrawlAlreadyRunning => "crawl already running".into(),
        }
    }

    pub fn is_auth_related(&self) -> bool {
        matches!(
            self,
            DomainError::Llm {
                kind: LlmErrorKind::Auth,
                ..
            }
        )
    }

    pub fn is_provider_quota(&self) -> bool {
        matches!(
            self,
            DomainError::Llm {
                kind: LlmErrorKind::InsufficientQuota,
                ..
            }
        )
    }
}

pub type DomainResult<T> = Result<T, DomainError>;
