use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;

use crate::adapters::telegram::keyboards::{
    about_packs_keyboard, main_menu, schedules_keyboard, settings_keyboard,
};
use crate::adapters::telegram::state::{send_long, AppState};
use crate::application::FsmState;
use crate::domain::{Locale, TelegramUserId, FREE_CRAWLS_PER_DAY, FREE_SCHEDULE_SLOTS};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Smart Hawk commands")]
pub enum Command {
    #[command(description = "Start / resume")]
    Start,
    #[command(description = "Crawl a URL")]
    Crawl,
    #[command(description = "Schedules")]
    Schedule,
    #[command(description = "Settings")]
    Settings,
    #[command(description = "History")]
    History,
    #[command(description = "Usage / quota")]
    Usage,
    #[command(description = "About & Stars")]
    About,
    #[command(description = "Cancel current wizard")]
    Cancel,
}

pub async fn handle(bot: Bot, msg: Message, cmd: Command, state: AppState) -> anyhow::Result<()> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let tg_id = TelegramUserId::new(from.id.0 as i64);
    let name = from
        .username
        .clone()
        .or_else(|| from.first_name.clone().into())
        .unwrap_or_else(|| "hunter".into());
    let lang = from.language_code.as_deref();

    let ensured = state
        .onboard
        .ensure_user(tg_id, &name, lang)
        .await
        .map_err(anyhow::Error::msg)?;
    let mut user = ensured.user;
    let locale = user.locale;

    match cmd {
        Command::Start => {
            if ensured.is_new
                || matches!(
                    ensured.fsm,
                    FsmState::OnboardingAskUsername { .. }
                        | FsmState::OnboardingChooseProvider
                        | FsmState::OnboardingAskBaseUrl { .. }
                        | FsmState::OnboardingAskApiKey { .. }
                        | FsmState::OnboardingPickModel { .. }
                )
            {
                let fsm = match ensured.fsm {
                    FsmState::Idle if ensured.is_new => FsmState::OnboardingAskUsername {
                        default_name: user.display_name.clone(),
                    },
                    other => other,
                };
                if matches!(fsm, FsmState::OnboardingAskUsername { .. }) {
                    let _ = state.onboard.save_fsm(tg_id, &fsm).await;
                }
                resume_onboarding(&bot, &msg, &state, locale, &fsm).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    state.i18n.t(
                        locale,
                        "welcome-back",
                        &[("name", user.display_name.clone())],
                    ),
                )
                .reply_markup(main_menu(&state.i18n, locale))
                .await?;
            }
        }
        Command::Cancel => {
            state
                .onboard
                .clear_fsm(tg_id)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(msg.chat.id, state.i18n.t0(locale, "mission-aborted"))
                .reply_markup(main_menu(&state.i18n, locale))
                .await?;
        }
        Command::Crawl => {
            begin_crawl(&bot, &msg, &state, &user).await?;
        }
        Command::Schedule => {
            show_schedules(&bot, &msg, &state, &user).await?;
        }
        Command::Settings => {
            bot.send_message(msg.chat.id, state.i18n.t0(locale, "settings-hub"))
                .reply_markup(settings_keyboard(&state.i18n, locale))
                .await?;
        }
        Command::History => {
            show_history(&bot, &msg, &state, &user).await?;
        }
        Command::Usage => {
            show_usage(&bot, &msg, &state, &user).await?;
        }
        Command::About => {
            show_about(&bot, &msg, &state, locale).await?;
        }
    }

    let _ = &mut user;
    Ok(())
}

pub async fn resume_onboarding(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    locale: Locale,
    fsm: &FsmState,
) -> anyhow::Result<()> {
    match fsm {
        FsmState::OnboardingAskUsername { default_name } => {
            bot.send_message(msg.chat.id, state.i18n.t0(locale, "welcome-new"))
                .await?;
            bot.send_message(
                msg.chat.id,
                state.i18n.t0(locale, "onboarding-ask-username"),
            )
            .reply_markup(crate::adapters::telegram::keyboards::default_name_keyboard(
                &state.i18n,
                locale,
                default_name,
            ))
            .await?;
        }
        FsmState::OnboardingChooseProvider | FsmState::SettingsAiChooseProvider => {
            let prefix = if matches!(fsm, FsmState::SettingsAiChooseProvider) {
                "settings_ai"
            } else {
                "onboard"
            };
            bot.send_message(
                msg.chat.id,
                state.i18n.t0(locale, "onboarding-choose-provider"),
            )
            .reply_markup(crate::adapters::telegram::keyboards::provider_keyboard(
                &state.i18n,
                locale,
                prefix,
            ))
            .await?;
        }
        FsmState::OnboardingAskBaseUrl { .. } | FsmState::SettingsAiAskBaseUrl { .. } => {
            bot.send_message(
                msg.chat.id,
                state.i18n.t0(locale, "onboarding-ask-base-url"),
            )
            .await?;
        }
        FsmState::OnboardingAskApiKey { .. } | FsmState::SettingsAiAskApiKey { .. } => {
            bot.send_message(msg.chat.id, state.i18n.t0(locale, "onboarding-ask-api-key"))
                .await?;
        }
        FsmState::OnboardingPickModel { models, page, .. }
        | FsmState::SettingsAiPickModel { models, page, .. } => {
            let (slice, page, pages) =
                crate::application::OnboardService::page_models(models, *page);
            let prefix = if matches!(fsm, FsmState::SettingsAiPickModel { .. }) {
                "settings_ai"
            } else {
                "onboard"
            };
            bot.send_message(
                msg.chat.id,
                state.i18n.t(
                    locale,
                    "onboarding-pick-model",
                    &[
                        ("page", (page + 1).to_string()),
                        ("pages", pages.to_string()),
                    ],
                ),
            )
            .reply_markup(crate::adapters::telegram::keyboards::models_keyboard(
                models, page, pages, prefix,
            ))
            .await?;
            let _ = slice;
        }
        _ => {
            bot.send_message(msg.chat.id, state.i18n.t0(locale, "main-menu-hint"))
                .reply_markup(main_menu(&state.i18n, locale))
                .await?;
        }
    }
    Ok(())
}

pub async fn begin_crawl(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user: &crate::domain::User,
) -> anyhow::Result<()> {
    let locale = user.locale;
    if let Err(e) = user.ensure_ready_to_crawl() {
        bot.send_message(msg.chat.id, state.i18n.t0(locale, "crawl-need-onboarding"))
            .await?;
        let _ = e;
        return Ok(());
    }
    if user
        .credits
        .total_crawl_budget_hint(state.onboard.clock.now())
        == 0
    {
        bot.send_message(msg.chat.id, state.i18n.t0(locale, "crawl-quota"))
            .await?;
        return Ok(());
    }
    state
        .onboard
        .save_fsm(user.telegram_user_id, &FsmState::CrawlAskUrl)
        .await
        .map_err(anyhow::Error::msg)?;
    bot.send_message(msg.chat.id, state.i18n.t0(locale, "crawl-ask-url"))
        .await?;
    Ok(())
}

pub async fn show_schedules(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user: &crate::domain::User,
) -> anyhow::Result<()> {
    let (list, used, max) = state
        .schedules
        .list(user.id)
        .await
        .map_err(anyhow::Error::msg)?;
    let (text, kb) = schedules_keyboard(&state.i18n, user.locale, &list, used, max);
    bot.send_message(msg.chat.id, text).reply_markup(kb).await?;
    Ok(())
}

pub async fn show_history(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user: &crate::domain::User,
) -> anyhow::Result<()> {
    let entries = state
        .history
        .list(user.id, 10, 0)
        .await
        .map_err(anyhow::Error::msg)?;
    if entries.is_empty() {
        bot.send_message(msg.chat.id, state.i18n.t0(user.locale, "history-empty"))
            .await?;
        return Ok(());
    }
    let mut text = state.i18n.t0(user.locale, "history-title");
    text.push('\n');
    let mut buttons = Vec::new();
    for e in &entries {
        let status = if e.status == crate::domain::CrawlStatus::Succeeded {
            "✅"
        } else {
            "❌"
        };
        let line = state.i18n.t(
            user.locale,
            "history-item",
            &[
                ("time", e.occurred_at.format("%Y-%m-%d %H:%M").to_string()),
                ("status", status.into()),
                ("url", e.start_url.clone()),
                ("prompt", e.prompt_snippet.clone()),
            ],
        );
        text.push('\n');
        text.push_str(&line);
        buttons.push((
            e.id.to_string(),
            format!(
                "{} {}",
                status,
                e.prompt_snippet.chars().take(30).collect::<String>()
            ),
        ));
    }
    bot.send_message(msg.chat.id, text)
        .reply_markup(crate::adapters::telegram::keyboards::history_keyboard(
            &buttons,
        ))
        .await?;
    Ok(())
}

pub async fn show_usage(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user: &crate::domain::User,
) -> anyhow::Result<()> {
    use crate::adapters::telegram::keyboards::usage_packs_hint_keyboard;
    use crate::domain::FREE_CRAWLS_PER_DAY;

    let now = state.onboard.clock.now();
    let free_left = user.credits.free_remaining(now);
    let free_used = FREE_CRAWLS_PER_DAY.saturating_sub(free_left);
    let (_, active, max) = state
        .schedules
        .list(user.id)
        .await
        .map_err(anyhow::Error::msg)?;

    let (provider, model, verified) = match &user.ai_config {
        Some(ai) => (
            ai.provider.display_name().to_string(),
            ai.model_id.as_str().to_string(),
            if ai.is_verified() {
                state.i18n.t0(user.locale, "usage-verified")
            } else {
                state.i18n.t0(user.locale, "usage-unverified")
            },
        ),
        None => (
            state.i18n.t0(user.locale, "usage-ai-none"),
            "—".into(),
            state.i18n.t0(user.locale, "usage-unverified"),
        ),
    };

    let title = state.i18n.t0(user.locale, "usage-title");
    let body = state.i18n.t(
        user.locale,
        "usage-body",
        &[
            ("free_used", free_used.to_string()),
            ("free_max", FREE_CRAWLS_PER_DAY.to_string()),
            ("free_left", free_left.to_string()),
            ("bonus", user.credits.bonus_crawl_credits.to_string()),
            ("active_sched", active.to_string()),
            ("max_sched", max.to_string()),
            ("bonus_slots", user.credits.bonus_schedule_slots.to_string()),
            (
                "total",
                user.credits.total_crawl_budget_hint(now).to_string(),
            ),
            ("provider", provider),
            ("model", model),
            ("verified", verified),
        ],
    );
    bot.send_message(msg.chat.id, format!("{title}\n\n{body}"))
        .reply_markup(usage_packs_hint_keyboard(&state.i18n, user.locale))
        .await?;
    Ok(())
}

pub async fn show_about(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    locale: Locale,
) -> anyhow::Result<()> {
    let text = state.i18n.t(
        locale,
        "about-body",
        &[
            ("landing", state.config.about_landing_url.clone()),
            ("github", state.config.about_github_url.clone()),
            ("free_crawls", FREE_CRAWLS_PER_DAY.to_string()),
            ("free_schedules", FREE_SCHEDULE_SLOTS.to_string()),
        ],
    );
    bot.send_message(msg.chat.id, text)
        .reply_markup(about_packs_keyboard())
        .await?;
    Ok(())
}

pub async fn map_domain_err(
    bot: &Bot,
    chat_id: ChatId,
    state: &AppState,
    locale: Locale,
    err: crate::domain::DomainError,
) -> anyhow::Result<()> {
    use crate::domain::DomainError;
    let text = if let Some(key) = err.i18n_key() {
        state.i18n.t0(locale, key)
    } else {
        match &err {
            DomainError::InvalidUrl(d) => {
                state
                    .i18n
                    .t(locale, "error-invalid-url", &[("detail", d.clone())])
            }
            DomainError::Validation(d) => {
                state
                    .i18n
                    .t(locale, "error-validation", &[("detail", d.clone())])
            }
            DomainError::FetchFailed(d) => {
                state
                    .i18n
                    .t(locale, "error-fetch", &[("detail", d.clone())])
            }
            other => state
                .i18n
                .t(locale, "error-generic", &[("detail", other.user_message())]),
        }
    };
    send_long(bot, chat_id, &text).await?;
    Ok(())
}
