use teloxide::prelude::*;
use teloxide::types::ParseMode;

use super::commands::{
    begin_crawl, map_domain_err, resume_onboarding, show_about, show_history, show_schedules,
    show_usage,
};
use crate::adapters::telegram::keyboards::{
    delivery_keyboard, interval_keyboard, main_menu, recurrence_keyboard,
};
use crate::adapters::telegram::state::{send_long, AppState};
use crate::application::EnqueueCrawl;
use crate::application::FsmState;
use crate::domain::{CrawlSource, DeliveryFlags, Recurrence, TelegramUserId};

pub async fn handle_text(bot: Bot, msg: Message, state: AppState) -> anyhow::Result<()> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let Some(text) = msg.text() else {
        return Ok(());
    };

    let tg_id = TelegramUserId::new(from.id.0 as i64);
    let name = from
        .username
        .clone()
        .unwrap_or_else(|| from.first_name.clone());
    let ensured = state
        .onboard
        .ensure_user(tg_id, &name, from.language_code.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    let mut user = ensured.user;
    let locale = user.locale;
    let i18n = &state.i18n;

    // Menu buttons
    if text == i18n.t0(locale, "btn-crawl") {
        return begin_crawl(&bot, &msg, &state, &user).await;
    }
    if text == i18n.t0(locale, "btn-schedule") {
        return show_schedules(&bot, &msg, &state, &user).await;
    }
    if text == i18n.t0(locale, "btn-settings") {
        bot.send_message(msg.chat.id, i18n.t0(locale, "settings-hub"))
            .reply_markup(crate::adapters::telegram::keyboards::settings_keyboard(
                i18n, locale,
            ))
            .await?;
        return Ok(());
    }
    if text == i18n.t0(locale, "btn-history") {
        return show_history(&bot, &msg, &state, &user).await;
    }
    if text == i18n.t0(locale, "btn-usage") {
        return show_usage(&bot, &msg, &state, &user).await;
    }
    if text == i18n.t0(locale, "btn-about") {
        return show_about(&bot, &msg, &state, locale).await;
    }

    let fsm = state
        .onboard
        .conversations
        .get(tg_id)
        .await
        .map_err(anyhow::Error::msg)?
        .map(|r| FsmState::from_stored(&r.state_kind, &r.state_payload))
        .transpose()
        .map_err(anyhow::Error::msg)?
        .unwrap_or(FsmState::Idle);

    match fsm {
        FsmState::OnboardingAskUsername { .. } => {
            let next = state
                .onboard
                .set_username(&mut user, text.to_string())
                .await
                .map_err(anyhow::Error::msg)?;
            resume_onboarding(&bot, &msg, &state, locale, &next).await?;
        }
        FsmState::OnboardingAskBaseUrl { provider } => {
            match state
                .onboard
                .set_base_url(&user, provider, text.to_string(), false)
                .await
            {
                Ok(next) => resume_onboarding(&bot, &msg, &state, locale, &next).await?,
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::SettingsAiAskBaseUrl { provider } => {
            match state
                .onboard
                .set_base_url(&user, provider, text.to_string(), true)
                .await
            {
                Ok(next) => resume_onboarding(&bot, &msg, &state, locale, &next).await?,
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::OnboardingAskApiKey { provider, base_url } => {
            handle_api_key(
                &bot, &msg, &state, &mut user, provider, base_url, text, false,
            )
            .await?;
        }
        FsmState::SettingsAiAskApiKey { provider, base_url } => {
            handle_api_key(
                &bot, &msg, &state, &mut user, provider, base_url, text, true,
            )
            .await?;
        }
        FsmState::CrawlAskUrl => match state.crawls.validate_url(text).await {
            Ok(url) => {
                let next = FsmState::CrawlAskPrompt {
                    url: url.to_string(),
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(msg.chat.id, i18n.t0(locale, "crawl-ask-prompt"))
                    .await?;
            }
            Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
        },
        FsmState::CrawlAskPrompt { url } => {
            match state
                .crawls
                .enqueue(EnqueueCrawl {
                    user_id: user.id,
                    url,
                    prompt: text.to_string(),
                    source: CrawlSource::Interactive,
                    schedule_id: None,
                })
                .await
            {
                Ok(job) => {
                    state
                        .onboard
                        .save_fsm(tg_id, &FsmState::Idle)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(msg.chat.id, i18n.t0(locale, "crawl-started"))
                        .reply_markup(main_menu(i18n, locale))
                        .await?;
                    let _ = state.crawl_tx.send(job.id).await;
                }
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::ScheduleAskUrl => match state.crawls.validate_url(text).await {
            Ok(url) => {
                let next = FsmState::ScheduleAskPrompt {
                    url: url.to_string(),
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(msg.chat.id, i18n.t0(locale, "schedule-ask-prompt"))
                    .await?;
            }
            Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
        },
        FsmState::ScheduleAskPrompt { url } => {
            let next = FsmState::SchedulePickRecurrence {
                url,
                prompt: text.to_string(),
            };
            state
                .onboard
                .save_fsm(tg_id, &next)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(msg.chat.id, i18n.t0(locale, "schedule-recurrence"))
                .reply_markup(recurrence_keyboard(i18n, locale))
                .await?;
        }
        FsmState::ScheduleAskDailyTime { url, prompt } => match crate::domain::parse_hhmm(text) {
            Ok(t) => {
                let rec = Recurrence::daily(t);
                let rec_json = serde_json::to_string(&rec).unwrap();
                let next = FsmState::ScheduleDelivery {
                    url,
                    prompt,
                    recurrence_json: rec_json,
                    send_chat: true,
                    send_email: false,
                    send_trigger: true,
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(msg.chat.id, i18n.t0(locale, "schedule-delivery"))
                    .reply_markup(delivery_keyboard(true, false, true))
                    .await?;
            }
            Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
        },
        FsmState::ScheduleWeeklyTime { url, prompt, days } => {
            match crate::domain::parse_hhmm(text) {
                Ok(t) => {
                    let weekdays: Result<Vec<_>, _> = days
                        .iter()
                        .map(|d| match d.as_str() {
                            "mon" => Ok(chrono::Weekday::Mon),
                            "tue" => Ok(chrono::Weekday::Tue),
                            "wed" => Ok(chrono::Weekday::Wed),
                            "thu" => Ok(chrono::Weekday::Thu),
                            "fri" => Ok(chrono::Weekday::Fri),
                            "sat" => Ok(chrono::Weekday::Sat),
                            "sun" => Ok(chrono::Weekday::Sun),
                            _ => Err(crate::domain::DomainError::Validation("day".into())),
                        })
                        .collect();
                    match weekdays.and_then(|d| Recurrence::weekly(d, t)) {
                        Ok(rec) => {
                            let rec_json = serde_json::to_string(&rec).unwrap();
                            let next = FsmState::ScheduleDelivery {
                                url,
                                prompt,
                                recurrence_json: rec_json,
                                send_chat: true,
                                send_email: false,
                                send_trigger: true,
                            };
                            state
                                .onboard
                                .save_fsm(tg_id, &next)
                                .await
                                .map_err(anyhow::Error::msg)?;
                            bot.send_message(msg.chat.id, i18n.t0(locale, "schedule-delivery"))
                                .reply_markup(delivery_keyboard(true, false, true))
                                .await?;
                        }
                        Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
                    }
                }
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::SettingsAskName => {
            match state.settings.set_name(&mut user, text.to_string()).await {
                Ok(()) => {
                    state
                        .onboard
                        .save_fsm(tg_id, &FsmState::Idle)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(msg.chat.id, i18n.t0(locale, "settings-saved"))
                        .reply_markup(main_menu(i18n, locale))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::SettingsAskEmail => {
            let email = if text.trim().eq_ignore_ascii_case("clear") {
                None
            } else {
                Some(text.to_string())
            };
            match state.settings.set_email(&mut user, email).await {
                Ok(()) => {
                    state
                        .onboard
                        .save_fsm(tg_id, &FsmState::Idle)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(msg.chat.id, i18n.t0(locale, "settings-saved"))
                        .reply_markup(main_menu(i18n, locale))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::SettingsAskTimezone => {
            match state
                .settings
                .set_timezone(&mut user, text.to_string())
                .await
            {
                Ok(()) => {
                    state
                        .onboard
                        .save_fsm(tg_id, &FsmState::Idle)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(msg.chat.id, i18n.t0(locale, "settings-saved"))
                        .reply_markup(main_menu(i18n, locale))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, msg.chat.id, &state, locale, e).await?,
            }
        }
        FsmState::SettingsConfirmDelete => {
            if text.trim() == "DELETE" {
                state
                    .settings
                    .hard_delete(user.id)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(msg.chat.id, i18n.t0(locale, "settings-deleted"))
                    .await?;
            } else {
                state
                    .onboard
                    .save_fsm(tg_id, &FsmState::Idle)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(msg.chat.id, i18n.t0(locale, "mission-aborted"))
                    .reply_markup(main_menu(i18n, locale))
                    .await?;
            }
        }
        FsmState::Idle => {
            bot.send_message(msg.chat.id, i18n.t0(locale, "main-menu-hint"))
                .reply_markup(main_menu(i18n, locale))
                .await?;
        }
        other => {
            // Unexpected free text for button-driven states
            resume_onboarding(&bot, &msg, &state, locale, &other).await?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_api_key(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user: &mut crate::domain::User,
    provider: crate::domain::ProviderKind,
    base_url: String,
    text: &str,
    settings_mode: bool,
) -> anyhow::Result<()> {
    let locale = user.locale;
    let loading = bot
        .send_message(
            msg.chat.id,
            state.i18n.t0(locale, "onboarding-loading-models"),
        )
        .await?;

    // Best-effort delete of the key message
    let _ = bot.delete_message(msg.chat.id, msg.id).await;

    match state
        .onboard
        .set_api_key_and_list_models(user, provider, base_url, text.to_string(), settings_mode)
        .await
        .map_err(anyhow::Error::msg)?
    {
        Ok(next) => {
            let _ = bot
                .edit_message_text(
                    loading.chat.id,
                    loading.id,
                    state.i18n.t0(locale, "onboarding-key-stored"),
                )
                .await;
            resume_onboarding(bot, msg, state, locale, &next).await?;
        }
        Err((back, err)) => {
            let _ = state.onboard.save_fsm(user.telegram_user_id, &back).await;
            let text = if err.is_auth_related() {
                state.i18n.t0(locale, "onboarding-auth-failed")
            } else if err.is_provider_quota() {
                state.i18n.t0(locale, "error-llm-quota")
            } else if let Some(key) = err.i18n_key() {
                state.i18n.t0(locale, key)
            } else {
                state.i18n.t(
                    locale,
                    "onboarding-provider-failed",
                    &[("detail", err.user_message())],
                )
            };
            let _ = bot
                .edit_message_text(loading.chat.id, loading.id, &text)
                .await;
            resume_onboarding(bot, msg, state, locale, &back).await?;
        }
    }
    Ok(())
}

// silence unused imports in some builds
#[allow(dead_code)]
fn _unused() {
    let _ = ParseMode::MarkdownV2;
    let _ = DeliveryFlags::chat_only();
    let _ = interval_keyboard();
    let _ = send_long;
}
