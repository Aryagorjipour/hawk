use serde::{Deserialize, Serialize};

use crate::domain::{ProviderKind, ScheduleId};

/// Finite state machine for multi-step Telegram dialogs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FsmState {
    Idle,

    OnboardingAskUsername {
        default_name: String,
    },
    OnboardingChooseProvider,
    OnboardingAskBaseUrl {
        provider: ProviderKind,
    },
    OnboardingAskApiKey {
        provider: ProviderKind,
        base_url: String,
    },
    OnboardingPickModel {
        provider: ProviderKind,
        base_url: String,
        page: usize,
        models: Vec<String>,
    },
    OnboardingVerify {
        provider: ProviderKind,
        base_url: String,
        model_id: String,
    },

    SettingsAiChooseProvider,
    SettingsAiAskBaseUrl {
        provider: ProviderKind,
    },
    SettingsAiAskApiKey {
        provider: ProviderKind,
        base_url: String,
    },
    SettingsAiPickModel {
        provider: ProviderKind,
        base_url: String,
        page: usize,
        models: Vec<String>,
    },
    SettingsAiVerify {
        provider: ProviderKind,
        base_url: String,
        model_id: String,
    },

    CrawlAskUrl,
    CrawlAskPrompt {
        url: String,
    },

    ScheduleAskUrl,
    ScheduleAskPrompt {
        url: String,
    },
    SchedulePickRecurrence {
        url: String,
        prompt: String,
    },
    SchedulePickInterval {
        url: String,
        prompt: String,
    },
    ScheduleAskDailyTime {
        url: String,
        prompt: String,
    },
    ScheduleWeeklyDays {
        url: String,
        prompt: String,
        days: Vec<String>,
    },
    ScheduleWeeklyTime {
        url: String,
        prompt: String,
        days: Vec<String>,
    },
    ScheduleDelivery {
        url: String,
        prompt: String,
        recurrence_json: String,
        send_chat: bool,
        send_email: bool,
        send_trigger: bool,
    },

    SettingsAskName,
    SettingsAskEmail,
    SettingsAskTimezone,
    SettingsConfirmDelete,

    ScheduleView {
        schedule_id: ScheduleId,
    },
}

impl FsmState {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::OnboardingAskUsername { .. } => "onboarding_ask_username",
            Self::OnboardingChooseProvider => "onboarding_choose_provider",
            Self::OnboardingAskBaseUrl { .. } => "onboarding_ask_base_url",
            Self::OnboardingAskApiKey { .. } => "onboarding_ask_api_key",
            Self::OnboardingPickModel { .. } => "onboarding_pick_model",
            Self::OnboardingVerify { .. } => "onboarding_verify",
            Self::SettingsAiChooseProvider => "settings_ai_choose_provider",
            Self::SettingsAiAskBaseUrl { .. } => "settings_ai_ask_base_url",
            Self::SettingsAiAskApiKey { .. } => "settings_ai_ask_api_key",
            Self::SettingsAiPickModel { .. } => "settings_ai_pick_model",
            Self::SettingsAiVerify { .. } => "settings_ai_verify",
            Self::CrawlAskUrl => "crawl_ask_url",
            Self::CrawlAskPrompt { .. } => "crawl_ask_prompt",
            Self::ScheduleAskUrl => "schedule_ask_url",
            Self::ScheduleAskPrompt { .. } => "schedule_ask_prompt",
            Self::SchedulePickRecurrence { .. } => "schedule_pick_recurrence",
            Self::SchedulePickInterval { .. } => "schedule_pick_interval",
            Self::ScheduleAskDailyTime { .. } => "schedule_ask_daily_time",
            Self::ScheduleWeeklyDays { .. } => "schedule_weekly_days",
            Self::ScheduleWeeklyTime { .. } => "schedule_weekly_time",
            Self::ScheduleDelivery { .. } => "schedule_delivery",
            Self::SettingsAskName => "settings_ask_name",
            Self::SettingsAskEmail => "settings_ask_email",
            Self::SettingsAskTimezone => "settings_ask_timezone",
            Self::SettingsConfirmDelete => "settings_confirm_delete",
            Self::ScheduleView { .. } => "schedule_view",
        }
    }

    pub fn to_payload(&self) -> crate::domain::DomainResult<String> {
        serde_json::to_string(self).map_err(|e| crate::domain::DomainError::Parse(e.to_string()))
    }

    pub fn from_stored(kind: &str, payload: &str) -> crate::domain::DomainResult<Self> {
        let _ = kind;
        serde_json::from_str(payload).map_err(|e| crate::domain::DomainError::Parse(e.to_string()))
    }
}

pub const MODELS_PER_PAGE: usize = 8;

pub fn model_page(models: &[String], page: usize) -> (Vec<String>, usize) {
    let pages = models.len().div_ceil(MODELS_PER_PAGE).max(1);
    let page = page.min(pages.saturating_sub(1));
    let start = page * MODELS_PER_PAGE;
    let end = (start + MODELS_PER_PAGE).min(models.len());
    (models[start..end].to_vec(), pages)
}
