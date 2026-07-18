use std::sync::Arc;

use tracing::info;

use crate::adapters::llm::build_llm_client_raw;
use crate::domain::{
    ApiKey, DomainError, DomainEvent, DomainResult, Locale, ModelId, ProviderKind, TelegramUserId,
    User, FREE_CRAWLS_PER_DAY, FREE_SCHEDULE_SLOTS,
};
use crate::infrastructure::EventBus;
use crate::ports::{Clock, ConversationRecord, ConversationRepository, SecretBox, UserRepository};

use super::conversation::{model_page, FsmState, MODELS_PER_PAGE};

pub struct OnboardService {
    pub users: Arc<dyn UserRepository>,
    pub conversations: Arc<dyn ConversationRepository>,
    pub secrets: Arc<dyn SecretBox>,
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
}

// conversations + clock are intentionally public for Telegram handlers

pub struct EnsureUserResult {
    pub user: User,
    pub is_new: bool,
    pub fsm: FsmState,
}

impl OnboardService {
    pub async fn ensure_user(
        &self,
        telegram_id: TelegramUserId,
        telegram_name: &str,
        language_code: Option<&str>,
    ) -> DomainResult<EnsureUserResult> {
        if let Some(user) = self.users.get_by_telegram_id(telegram_id).await? {
            let fsm = self
                .conversations
                .get(telegram_id)
                .await?
                .map(|r| FsmState::from_stored(&r.state_kind, &r.state_payload))
                .transpose()?
                .unwrap_or(FsmState::Idle);
            return Ok(EnsureUserResult {
                user,
                is_new: false,
                fsm,
            });
        }

        let locale = Locale::from_telegram_language_code(language_code);
        let default_name = if telegram_name.trim().is_empty() {
            format!("hunter-{}", telegram_id.get())
        } else {
            telegram_name.trim().to_string()
        };
        let now = self.clock.now();
        let user = User::new(telegram_id, default_name.clone(), locale, now)?;
        self.users.insert(&user).await?;

        let fsm = FsmState::OnboardingAskUsername {
            default_name: default_name.clone(),
        };
        self.save_fsm(telegram_id, &fsm).await?;

        info!(user_id = %user.id, "user_created");
        Ok(EnsureUserResult {
            user,
            is_new: true,
            fsm,
        })
    }

    pub async fn save_fsm(&self, telegram_id: TelegramUserId, fsm: &FsmState) -> DomainResult<()> {
        let rec = ConversationRecord {
            telegram_user_id: telegram_id,
            state_kind: fsm.kind_name().to_string(),
            state_payload: fsm.to_payload()?,
            updated_at: self.clock.now(),
        };
        self.conversations.upsert(&rec).await
    }

    pub async fn clear_fsm(&self, telegram_id: TelegramUserId) -> DomainResult<()> {
        self.conversations.delete(telegram_id).await
    }

    pub async fn set_username(&self, user: &mut User, name: String) -> DomainResult<FsmState> {
        let now = self.clock.now();
        user.set_display_name(name, now)?;
        self.users.update(user).await?;
        let next = FsmState::OnboardingChooseProvider;
        self.save_fsm(user.telegram_user_id, &next).await?;
        Ok(next)
    }

    pub async fn choose_provider(
        &self,
        user: &User,
        provider: ProviderKind,
        settings_mode: bool,
    ) -> DomainResult<FsmState> {
        let next = if provider.requires_custom_base_url() {
            if settings_mode {
                FsmState::SettingsAiAskBaseUrl { provider }
            } else {
                FsmState::OnboardingAskBaseUrl { provider }
            }
        } else {
            let base = provider.default_base_url().unwrap().to_string();
            if settings_mode {
                FsmState::SettingsAiAskApiKey {
                    provider,
                    base_url: base,
                }
            } else {
                FsmState::OnboardingAskApiKey {
                    provider,
                    base_url: base,
                }
            }
        };
        self.save_fsm(user.telegram_user_id, &next).await?;
        Ok(next)
    }

    pub async fn set_base_url(
        &self,
        user: &User,
        provider: ProviderKind,
        base_url: String,
        settings_mode: bool,
    ) -> DomainResult<FsmState> {
        let url = crate::domain::validate_base_url(&base_url)?;
        let base = url.to_string().trim_end_matches('/').to_string();
        let next = if settings_mode {
            FsmState::SettingsAiAskApiKey {
                provider,
                base_url: base,
            }
        } else {
            FsmState::OnboardingAskApiKey {
                provider,
                base_url: base,
            }
        };
        self.save_fsm(user.telegram_user_id, &next).await?;
        Ok(next)
    }

    pub async fn set_api_key_and_list_models(
        &self,
        user: &mut User,
        provider: ProviderKind,
        base_url: String,
        api_key_raw: String,
        settings_mode: bool,
    ) -> DomainResult<Result<FsmState, (FsmState, DomainError)>> {
        let api_key = match ApiKey::new(api_key_raw) {
            Ok(k) => k,
            Err(e) => {
                let back = key_step(provider, base_url, settings_mode);
                return Ok(Err((back, e)));
            }
        };

        let client = match build_llm_client_raw(provider, &base_url, &api_key) {
            Ok(c) => c,
            Err(e) => {
                let back = if e.is_auth_related() {
                    key_step(provider, base_url, settings_mode)
                } else {
                    provider_step(settings_mode)
                };
                return Ok(Err((back, e)));
            }
        };

        let models = match client.list_models().await {
            Ok(m) => m,
            Err(e) => {
                let back = if e.is_auth_related() {
                    key_step(provider, base_url, settings_mode)
                } else {
                    provider_step(settings_mode)
                };
                let _ = self.save_fsm(user.telegram_user_id, &back).await;
                return Ok(Err((back, e)));
            }
        };

        let blob = self.secrets.encrypt(&api_key)?;
        // Temporarily store encrypted key with placeholder model until pick
        let placeholder = ModelId::new(models.first().map(|m| m.id.as_str()).unwrap_or("pending"))?;
        user.set_ai_partial(
            provider,
            Some(base_url.clone()),
            blob,
            placeholder,
            self.clock.now(),
        )?;
        self.users.update(user).await?;

        let ids: Vec<String> = models.into_iter().map(|m| m.id).collect();
        let next = if settings_mode {
            FsmState::SettingsAiPickModel {
                provider,
                base_url,
                page: 0,
                models: ids,
            }
        } else {
            FsmState::OnboardingPickModel {
                provider,
                base_url,
                page: 0,
                models: ids,
            }
        };
        self.save_fsm(user.telegram_user_id, &next).await?;
        Ok(Ok(next))
    }

    pub async fn select_model_and_verify(
        &self,
        user: &mut User,
        provider: ProviderKind,
        base_url: String,
        model_id: String,
        settings_mode: bool,
    ) -> DomainResult<Result<(), (FsmState, DomainError)>> {
        let model = ModelId::new(model_id.clone())?;
        let ai = user.ai_config.as_ref().ok_or(DomainError::AiNotVerified)?;
        let api_key = self.secrets.decrypt(&ai.api_key)?;
        let client = build_llm_client_raw(provider, &base_url, &api_key)?;

        if let Err(e) = client.probe_connection(&model).await {
            let back = if e.is_auth_related() {
                key_step(provider, base_url, settings_mode)
            } else {
                provider_step(settings_mode)
            };
            let _ = self.save_fsm(user.telegram_user_id, &back).await;
            return Ok(Err((back, e)));
        }

        let blob = ai.api_key.clone();
        user.set_ai_partial(provider, Some(base_url), blob, model, self.clock.now())?;
        user.mark_ai_verified(self.clock.now())?;
        self.users.update(user).await?;
        self.save_fsm(user.telegram_user_id, &FsmState::Idle)
            .await?;
        self.events.publish(DomainEvent::UserOnboarded {
            user_id: user.id,
            at: self.clock.now(),
        });
        Ok(Ok(()))
    }

    pub fn success_args(user: &User) -> Vec<(&'static str, String)> {
        vec![
            ("name", user.display_name.clone()),
            ("free_crawls", FREE_CRAWLS_PER_DAY.to_string()),
            ("free_schedules", FREE_SCHEDULE_SLOTS.to_string()),
        ]
    }

    pub fn page_models(models: &[String], page: usize) -> (Vec<String>, usize, usize) {
        let (slice, pages) = model_page(models, page);
        (slice, page.min(pages.saturating_sub(1)), pages)
    }
}

fn key_step(provider: ProviderKind, base_url: String, settings: bool) -> FsmState {
    if settings {
        FsmState::SettingsAiAskApiKey { provider, base_url }
    } else {
        FsmState::OnboardingAskApiKey { provider, base_url }
    }
}

fn provider_step(settings: bool) -> FsmState {
    if settings {
        FsmState::SettingsAiChooseProvider
    } else {
        FsmState::OnboardingChooseProvider
    }
}

#[allow(dead_code)]
pub fn models_per_page() -> usize {
    MODELS_PER_PAGE
}
