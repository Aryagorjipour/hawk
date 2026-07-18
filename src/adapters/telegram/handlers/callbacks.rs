use teloxide::prelude::*;
use teloxide::types::LabeledPrice;

use super::commands::{
    map_domain_err, resume_onboarding, show_about, show_history, show_schedules,
};
use crate::adapters::telegram::keyboards::{
    delivery_keyboard, interval_keyboard, main_menu, models_keyboard, recurrence_keyboard,
    schedule_detail_keyboard, settings_keyboard, timezone_keyboard,
};
use crate::adapters::telegram::state::AppState;
use crate::application::FsmState;
use crate::domain::{
    pack_by_id, DeliveryFlags, Locale, ProviderKind, Recurrence, ScheduleId, TelegramUserId,
};

pub async fn handle(bot: Bot, q: CallbackQuery, state: AppState) -> anyhow::Result<()> {
    let Some(data) = q.data.clone() else {
        return Ok(());
    };
    let from = q.from.clone();
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
    let chat_id = q
        .message
        .as_ref()
        .map(|m| m.chat().id)
        .unwrap_or(teloxide::types::ChatId(tg_id.get()));

    let _ = bot.answer_callback_query(q.id.clone()).await;

    let parts: Vec<&str> = data.split(':').collect();

    match parts.as_slice() {
        ["onboard", "default_name", name] => {
            let next = state
                .onboard
                .set_username(&mut user, name.to_string())
                .await
                .map_err(anyhow::Error::msg)?;
            if let Some(m) = q.message.as_ref() {
                let msg = m.regular_message().cloned();
                if let Some(msg) = msg {
                    resume_onboarding(&bot, &msg, &state, locale, &next).await?;
                }
            }
        }
        ["onboard", "provider", p] | ["settings_ai", "provider", p] => {
            let settings = parts[0] == "settings_ai";
            let provider = ProviderKind::parse(p).map_err(anyhow::Error::msg)?;
            let next = state
                .onboard
                .choose_provider(&user, provider, settings)
                .await
                .map_err(anyhow::Error::msg)?;
            send_fsm(&bot, chat_id, &state, locale, &next).await?;
        }
        ["onboard", "page", page_s] | ["settings_ai", "page", page_s] => {
            let settings = parts[0] == "settings_ai";
            let page: usize = page_s.parse().unwrap_or(0);
            let fsm = load_fsm(&state, tg_id).await?;
            let next = match fsm {
                FsmState::OnboardingPickModel {
                    provider,
                    base_url,
                    models,
                    ..
                } => FsmState::OnboardingPickModel {
                    provider,
                    base_url,
                    page,
                    models,
                },
                FsmState::SettingsAiPickModel {
                    provider,
                    base_url,
                    models,
                    ..
                } => FsmState::SettingsAiPickModel {
                    provider,
                    base_url,
                    page,
                    models,
                },
                other => other,
            };
            let _ = settings;
            state
                .onboard
                .save_fsm(tg_id, &next)
                .await
                .map_err(anyhow::Error::msg)?;
            send_fsm(&bot, chat_id, &state, locale, &next).await?;
        }
        ["onboard", "model", model] | ["settings_ai", "model", model] => {
            let settings = parts[0] == "settings_ai";
            let fsm = load_fsm(&state, tg_id).await?;
            let (provider, base_url) = match &fsm {
                FsmState::OnboardingPickModel {
                    provider, base_url, ..
                }
                | FsmState::SettingsAiPickModel {
                    provider, base_url, ..
                } => (*provider, base_url.clone()),
                _ => return Ok(()),
            };
            bot.send_message(chat_id, state.i18n.t0(locale, "onboarding-verifying"))
                .await?;
            match state
                .onboard
                .select_model_and_verify(&mut user, provider, base_url, model.to_string(), settings)
                .await
                .map_err(anyhow::Error::msg)?
            {
                Ok(()) => {
                    let args = crate::application::OnboardService::success_args(&user);
                    let text = state.i18n.t(locale, "onboarding-success", &args);
                    bot.send_message(chat_id, text)
                        .reply_markup(main_menu(&state.i18n, locale))
                        .await?;
                }
                Err((back, err)) => {
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
                    bot.send_message(chat_id, text).await?;
                    send_fsm(&bot, chat_id, &state, locale, &back).await?;
                }
            }
        }
        ["settings", "name"] => {
            state
                .onboard
                .save_fsm(tg_id, &FsmState::SettingsAskName)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(chat_id, state.i18n.t0(locale, "settings-name"))
                .await?;
        }
        ["settings", "email"] => {
            state
                .onboard
                .save_fsm(tg_id, &FsmState::SettingsAskEmail)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(
                chat_id,
                format!(
                    "{}\n(current: {})\nSend new email or `clear`.",
                    state.i18n.t0(locale, "settings-email"),
                    user.email.as_deref().unwrap_or("none")
                ),
            )
            .await?;
        }
        ["settings", "tz"] => {
            state
                .onboard
                .save_fsm(tg_id, &FsmState::SettingsAskTimezone)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(chat_id, state.i18n.t0(locale, "settings-timezone"))
                .reply_markup(timezone_keyboard())
                .await?;
        }
        ["settings", "setz", tz] => {
            match state.settings.set_timezone(&mut user, tz.to_string()).await {
                Ok(()) => {
                    state
                        .onboard
                        .save_fsm(tg_id, &FsmState::Idle)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(chat_id, state.i18n.t0(locale, "settings-saved"))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        ["settings", "ai"] => {
            let next = FsmState::SettingsAiChooseProvider;
            state
                .onboard
                .save_fsm(tg_id, &next)
                .await
                .map_err(anyhow::Error::msg)?;
            send_fsm(&bot, chat_id, &state, locale, &next).await?;
        }
        ["settings", "lang", code] => {
            let loc = Locale::parse(code);
            state
                .settings
                .set_locale(&mut user, loc)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(chat_id, state.i18n.t0(loc, "settings-saved"))
                .reply_markup(main_menu(&state.i18n, loc))
                .await?;
        }
        ["settings", "delete"] => {
            state
                .onboard
                .save_fsm(tg_id, &FsmState::SettingsConfirmDelete)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(chat_id, state.i18n.t0(locale, "settings-delete-confirm"))
                .await?;
        }
        ["sched", "new"] => {
            state
                .onboard
                .save_fsm(tg_id, &FsmState::ScheduleAskUrl)
                .await
                .map_err(anyhow::Error::msg)?;
            bot.send_message(chat_id, state.i18n.t0(locale, "schedule-ask-url"))
                .await?;
        }
        ["sched", "list"] => {
            if let Some(m) = q.message.as_ref().and_then(|m| m.regular_message()) {
                show_schedules(&bot, m, &state, &user).await?;
            }
        }
        ["sched", "open", id] => {
            let sid = ScheduleId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            match state.schedules.get(user.id, sid).await {
                Ok(s) => {
                    let status = if s.active {
                        state.i18n.t0(locale, "schedule-activate")
                    } else {
                        state.i18n.t0(locale, "schedule-deactivate")
                    };
                    let text = format!(
                        "🔗 {}\n📝 {}\n{}\n⏭ next: {}",
                        s.start_url,
                        s.user_prompt,
                        status,
                        s.next_run_at.format("%Y-%m-%d %H:%M UTC")
                    );
                    bot.send_message(chat_id, text)
                        .reply_markup(schedule_detail_keyboard(
                            &state.i18n,
                            locale,
                            &s.id.to_string(),
                            s.active,
                        ))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        ["sched", "run", id] => {
            let sid = ScheduleId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            match state.schedules.get(user.id, sid).await {
                Ok(s) => {
                    match state
                        .crawls
                        .enqueue(crate::application::EnqueueCrawl {
                            user_id: user.id,
                            url: s.start_url.clone(),
                            prompt: s.user_prompt.clone(),
                            source: crate::domain::CrawlSource::Schedule,
                            schedule_id: Some(s.id),
                        })
                        .await
                    {
                        Ok(job) => {
                            let _ = state.crawl_tx.send(job.id).await;
                            bot.send_message(chat_id, state.i18n.t0(locale, "schedule-triggered"))
                                .await?;
                        }
                        Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
                    }
                }
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        ["usage", "packs"] => {
            if let Some(m) = q.message.as_ref().and_then(|m| m.regular_message()) {
                show_about(&bot, m, &state, locale).await?;
            }
        }
        ["sched", "toggle", id] => {
            let sid = ScheduleId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            match state.schedules.get(user.id, sid).await {
                Ok(s) => match state.schedules.set_active(user.id, sid, !s.active).await {
                    Ok(updated) => {
                        bot.send_message(
                            chat_id,
                            state.i18n.t(
                                locale,
                                "schedule-toggled",
                                &[(
                                    "state",
                                    if updated.active {
                                        "active".into()
                                    } else {
                                        "inactive".into()
                                    },
                                )],
                            ),
                        )
                        .await?;
                    }
                    Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
                },
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        ["sched", "del", id] => {
            let sid = ScheduleId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            match state.schedules.delete(user.id, sid).await {
                Ok(()) => {
                    bot.send_message(chat_id, state.i18n.t0(locale, "schedule-deleted"))
                        .await?;
                }
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        ["sched", "hist", id] => {
            let sid = ScheduleId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            let entries = state
                .history
                .list_by_schedule(user.id, sid, 10)
                .await
                .map_err(anyhow::Error::msg)?;
            if entries.is_empty() {
                bot.send_message(chat_id, state.i18n.t0(locale, "history-empty"))
                    .await?;
            } else {
                let mut text = String::from("Schedule history:\n");
                for e in entries {
                    text.push_str(&format!(
                        "\n{} {} — {}",
                        e.occurred_at.format("%Y-%m-%d %H:%M"),
                        e.status.as_str(),
                        e.prompt_snippet
                    ));
                }
                bot.send_message(chat_id, text).await?;
            }
        }
        ["sched", "rec", "interval"] => {
            if let FsmState::SchedulePickRecurrence { url, prompt } =
                load_fsm(&state, tg_id).await?
            {
                let next = FsmState::SchedulePickInterval { url, prompt };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(chat_id, state.i18n.t0(locale, "schedule-pick-interval"))
                    .reply_markup(interval_keyboard())
                    .await?;
            }
        }
        ["sched", "rec", "daily"] => {
            if let FsmState::SchedulePickRecurrence { url, prompt } =
                load_fsm(&state, tg_id).await?
            {
                let next = FsmState::ScheduleAskDailyTime { url, prompt };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(
                    chat_id,
                    state.i18n.t(
                        locale,
                        "schedule-ask-time",
                        &[("tz", user.timezone.clone())],
                    ),
                )
                .await?;
            }
        }
        ["sched", "rec", "weekly"] => {
            if let FsmState::SchedulePickRecurrence { url, prompt } =
                load_fsm(&state, tg_id).await?
            {
                let next = FsmState::ScheduleWeeklyDays {
                    url,
                    prompt,
                    days: vec![],
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(chat_id, state.i18n.t0(locale, "schedule-pick-days"))
                    .reply_markup(weekday_keyboard(&[]))
                    .await?;
            }
        }
        ["sched", "int", mins] => {
            if let FsmState::SchedulePickInterval { url, prompt } = load_fsm(&state, tg_id).await? {
                let minutes: u32 = mins.parse().unwrap_or(60);
                match Recurrence::interval_minutes(minutes) {
                    Ok(rec) => {
                        let next = FsmState::ScheduleDelivery {
                            url,
                            prompt,
                            recurrence_json: serde_json::to_string(&rec).unwrap(),
                            send_chat: true,
                            send_email: false,
                            send_trigger: true,
                        };
                        state
                            .onboard
                            .save_fsm(tg_id, &next)
                            .await
                            .map_err(anyhow::Error::msg)?;
                        bot.send_message(chat_id, state.i18n.t0(locale, "schedule-delivery"))
                            .reply_markup(delivery_keyboard(true, false, true))
                            .await?;
                    }
                    Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
                }
            }
        }
        ["sched", "day", day] => {
            if let FsmState::ScheduleWeeklyDays {
                url,
                prompt,
                mut days,
            } = load_fsm(&state, tg_id).await?
            {
                if let Some(pos) = days.iter().position(|d| d == day) {
                    days.remove(pos);
                } else {
                    days.push(day.to_string());
                }
                let next = FsmState::ScheduleWeeklyDays {
                    url,
                    prompt,
                    days: days.clone(),
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(chat_id, state.i18n.t0(locale, "schedule-pick-days"))
                    .reply_markup(weekday_keyboard(&days))
                    .await?;
            }
        }
        ["sched", "days", "done"] => {
            if let FsmState::ScheduleWeeklyDays { url, prompt, days } =
                load_fsm(&state, tg_id).await?
            {
                if days.is_empty() {
                    bot.send_message(chat_id, "Pick at least one day.").await?;
                } else {
                    let next = FsmState::ScheduleWeeklyTime { url, prompt, days };
                    state
                        .onboard
                        .save_fsm(tg_id, &next)
                        .await
                        .map_err(anyhow::Error::msg)?;
                    bot.send_message(
                        chat_id,
                        state.i18n.t(
                            locale,
                            "schedule-ask-time",
                            &[("tz", user.timezone.clone())],
                        ),
                    )
                    .await?;
                }
            }
        }
        ["sched", "deliv", flag] => {
            if let FsmState::ScheduleDelivery {
                url,
                prompt,
                recurrence_json,
                mut send_chat,
                mut send_email,
                mut send_trigger,
            } = load_fsm(&state, tg_id).await?
            {
                match *flag {
                    "chat" => send_chat = !send_chat,
                    "email" => send_email = !send_email,
                    "trigger" => send_trigger = !send_trigger,
                    "save" => {
                        let rec: Recurrence =
                            serde_json::from_str(&recurrence_json).map_err(anyhow::Error::msg)?;
                        let delivery = DeliveryFlags {
                            send_chat,
                            send_email,
                            send_trigger_message: send_trigger,
                        };
                        match state
                            .schedules
                            .create(user.id, url, prompt, rec, delivery)
                            .await
                        {
                            Ok(s) => {
                                state
                                    .onboard
                                    .save_fsm(tg_id, &FsmState::Idle)
                                    .await
                                    .map_err(anyhow::Error::msg)?;
                                bot.send_message(
                                    chat_id,
                                    state.i18n.t(
                                        locale,
                                        "schedule-created",
                                        &[("next", s.next_run_at.to_rfc3339())],
                                    ),
                                )
                                .reply_markup(main_menu(&state.i18n, locale))
                                .await?;
                            }
                            Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
                        }
                        return Ok(());
                    }
                    _ => {}
                }
                let next = FsmState::ScheduleDelivery {
                    url,
                    prompt,
                    recurrence_json,
                    send_chat,
                    send_email,
                    send_trigger,
                };
                state
                    .onboard
                    .save_fsm(tg_id, &next)
                    .await
                    .map_err(anyhow::Error::msg)?;
                bot.send_message(chat_id, state.i18n.t0(locale, "schedule-delivery"))
                    .reply_markup(delivery_keyboard(send_chat, send_email, send_trigger))
                    .await?;
            }
        }
        ["pay", "pack", pack_id] => {
            let pack = pack_by_id(pack_id).map_err(anyhow::Error::msg)?;
            let title = format!("Smart Hawk {}", pack.id);
            let description = format!(
                "{} crawl credits + {} schedule slots",
                pack.credits, pack.schedule_slots
            );
            // Telegram Stars: currency XTR, amount = stars count
            let prices = vec![LabeledPrice {
                label: title.clone(),
                amount: pack.stars,
            }];
            bot.send_invoice(chat_id, title, description, pack.id, "", "XTR", prices)
                .await?;
        }
        ["hist", "view", id] => {
            let hid = crate::domain::HistoryEntryId::parse(id).map_err(|e| anyhow::anyhow!(e))?;
            match state.history.get(user.id, hid).await {
                Ok(e) => {
                    let body = e
                        .result_pretty
                        .or(e.error_detail)
                        .unwrap_or_else(|| "empty".into());
                    bot.send_message(chat_id, body).await?;
                }
                Err(e) => map_domain_err(&bot, chat_id, &state, locale, e).await?,
            }
        }
        _ => {
            tracing::debug!(data = %data, "unhandled_callback");
        }
    }

    let _ = settings_keyboard;
    let _ = recurrence_keyboard;
    let _ = models_keyboard;
    let _ = show_history;

    Ok(())
}

async fn load_fsm(state: &AppState, tg_id: TelegramUserId) -> anyhow::Result<FsmState> {
    Ok(state
        .onboard
        .conversations
        .get(tg_id)
        .await
        .map_err(anyhow::Error::msg)?
        .map(|r| FsmState::from_stored(&r.state_kind, &r.state_payload))
        .transpose()
        .map_err(anyhow::Error::msg)?
        .unwrap_or(FsmState::Idle))
}

async fn send_fsm(
    bot: &Bot,
    chat_id: ChatId,
    state: &AppState,
    locale: Locale,
    fsm: &FsmState,
) -> anyhow::Result<()> {
    // Build a synthetic message context via direct sends
    match fsm {
        FsmState::OnboardingChooseProvider | FsmState::SettingsAiChooseProvider => {
            let prefix = if matches!(fsm, FsmState::SettingsAiChooseProvider) {
                "settings_ai"
            } else {
                "onboard"
            };
            bot.send_message(chat_id, state.i18n.t0(locale, "onboarding-choose-provider"))
                .reply_markup(crate::adapters::telegram::keyboards::provider_keyboard(
                    &state.i18n,
                    locale,
                    prefix,
                ))
                .await?;
        }
        FsmState::OnboardingAskBaseUrl { .. } | FsmState::SettingsAiAskBaseUrl { .. } => {
            bot.send_message(chat_id, state.i18n.t0(locale, "onboarding-ask-base-url"))
                .await?;
        }
        FsmState::OnboardingAskApiKey { .. } | FsmState::SettingsAiAskApiKey { .. } => {
            bot.send_message(chat_id, state.i18n.t0(locale, "onboarding-ask-api-key"))
                .await?;
        }
        FsmState::OnboardingPickModel { models, page, .. }
        | FsmState::SettingsAiPickModel { models, page, .. } => {
            let (_, page, pages) = crate::application::OnboardService::page_models(models, *page);
            let prefix = if matches!(fsm, FsmState::SettingsAiPickModel { .. }) {
                "settings_ai"
            } else {
                "onboard"
            };
            bot.send_message(
                chat_id,
                state.i18n.t(
                    locale,
                    "onboarding-pick-model",
                    &[
                        ("page", (page + 1).to_string()),
                        ("pages", pages.to_string()),
                    ],
                ),
            )
            .reply_markup(models_keyboard(models, page, pages, prefix))
            .await?;
        }
        _ => {}
    }
    Ok(())
}

fn weekday_keyboard(selected: &[String]) -> teloxide::types::InlineKeyboardMarkup {
    use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};
    let days = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
    let mut rows = Vec::new();
    let mut row = Vec::new();
    for d in days {
        let mark = if selected.iter().any(|s| s == d) {
            format!("✓ {d}")
        } else {
            d.to_string()
        };
        row.push(InlineKeyboardButton::callback(
            mark,
            format!("sched:day:{d}"),
        ));
        if row.len() == 4 {
            rows.push(std::mem::take(&mut row));
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows.push(vec![InlineKeyboardButton::callback(
        "Done → set time",
        "sched:days:done",
    )]);
    InlineKeyboardMarkup::new(rows)
}
