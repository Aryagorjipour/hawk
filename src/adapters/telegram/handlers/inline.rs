use teloxide::prelude::*;
use teloxide::types::{
    InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText,
};

use crate::adapters::telegram::state::AppState;
use crate::application::EnqueueCrawl;
use crate::domain::{CrawlSource, TelegramUserId};

pub async fn handle_inline(bot: Bot, q: InlineQuery, state: AppState) -> anyhow::Result<()> {
    let tg_id = TelegramUserId::new(q.from.id.0 as i64);
    let name = q
        .from
        .username
        .clone()
        .unwrap_or_else(|| q.from.first_name.clone());
    let ensured = state
        .onboard
        .ensure_user(tg_id, &name, q.from.language_code.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    let user = ensured.user;
    let locale = user.locale;
    let query = q.query.trim();

    let mut results: Vec<InlineQueryResult> = Vec::new();

    if query.is_empty() || query == "?" {
        results.push(safe_article(
            "help_crawl",
            "🕊 Crawl from inline",
            state.i18n.t0(locale, "inline-help-crawl"),
        ));
        results.push(safe_article(
            "help_hist",
            "📜 History picker",
            state.i18n.t0(locale, "inline-help-history"),
        ));
    } else if query.starts_with('h') && (query.len() == 1 || query.starts_with("h ")) {
        if user.ensure_ready_to_crawl().is_err() {
            results.push(safe_article(
                "need_setup",
                "Setup required",
                state.i18n.t0(locale, "inline-need-setup"),
            ));
        } else {
            let entries = state
                .history
                .recent_successes(user.id, 10)
                .await
                .map_err(anyhow::Error::msg)?;
            for e in entries {
                let body = e
                    .result_pretty
                    .clone()
                    .unwrap_or_else(|| e.prompt_snippet.clone());
                // hist + uuid simple form is safe (hex only)
                let id = format!("h{}", e.id.as_uuid().as_simple());
                results.push(safe_article(
                    id,
                    format!("✅ {}", truncate(&e.prompt_snippet, 40)),
                    body,
                ));
            }
            if results.is_empty() {
                results.push(safe_article(
                    "hist_empty",
                    "No history",
                    state.i18n.t0(locale, "history-empty"),
                ));
            }
        }
    } else {
        let mut parts = query.split_whitespace();
        let url = parts.next().unwrap_or("").to_string();
        let prompt = {
            let rest: String = parts.collect::<Vec<_>>().join(" ");
            if rest.is_empty() {
                "Summarize the main content.".into()
            } else {
                rest
            }
        };

        if user.ensure_ready_to_crawl().is_err() {
            results.push(safe_article(
                "need_setup",
                "Setup required",
                state.i18n.t0(locale, "inline-need-setup"),
            ));
        } else if state.crawls.validate_url(&url).await.is_err() {
            results.push(safe_article(
                "bad_url",
                "Invalid URL",
                state
                    .i18n
                    .t(locale, "error-invalid-url", &[("detail", url)]),
            ));
        } else {
            // Safe short id + store payload (Telegram rejects urls/colons in result_id)
            let result_id = state
                .pending_inline
                .insert(user.id, tg_id.get(), url.clone(), prompt);
            results.push(safe_article(
                result_id,
                format!("🕊 Crawl {}", truncate(&url, 48)),
                state.i18n.t0(locale, "inline-started"),
            ));
        }
    }

    bot.answer_inline_query(q.id, results)
        .cache_time(1)
        .is_personal(true)
        .await?;
    Ok(())
}

pub async fn handle_chosen(
    _bot: Bot,
    result: ChosenInlineResult,
    state: AppState,
) -> anyhow::Result<()> {
    let result_id = result.result_id.as_str();

    // History pickers only inject text — nothing to enqueue
    if result_id.starts_with('h') && result_id.len() == 33 {
        return Ok(());
    }
    if matches!(
        result_id,
        "help_crawl" | "help_hist" | "need_setup" | "hist_empty" | "bad_url"
    ) {
        return Ok(());
    }

    let Some(pending) = state.pending_inline.take(result_id) else {
        tracing::debug!(%result_id, "inline_pending_miss");
        return Ok(());
    };

    let tg_id = TelegramUserId::new(result.from.id.0 as i64);
    let name = result
        .from
        .username
        .clone()
        .unwrap_or_else(|| result.from.first_name.clone());
    let ensured = state
        .onboard
        .ensure_user(tg_id, &name, result.from.language_code.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    let user = ensured.user;

    match state
        .crawls
        .enqueue(EnqueueCrawl {
            user_id: user.id,
            url: pending.url,
            prompt: pending.prompt,
            source: CrawlSource::Inline,
            schedule_id: None,
        })
        .await
    {
        Ok(job) => {
            let _ = state.crawl_tx.send(job.id).await;
            state
                .notify_chat(tg_id.get(), state.i18n.t0(user.locale, "inline-started"))
                .await;
        }
        Err(e) => {
            state.notify_chat(tg_id.get(), e.user_message()).await;
        }
    }
    Ok(())
}

/// Telegram: result id 1–64 bytes; avoid `: / ? &` etc. Use [A-Za-z0-9_].
fn safe_article(
    id: impl Into<String>,
    title: impl Into<String>,
    body: impl Into<String>,
) -> InlineQueryResult {
    let mut id = id.into();
    if id.len() > 64 {
        id.truncate(64);
    }
    // Strip anything outside a conservative charset
    id = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if id.is_empty() {
        id = "x".into();
    }

    let title = title.into();
    let body = body.into();
    let desc: String = body.chars().take(100).collect();
    InlineQueryResult::Article(
        InlineQueryResultArticle::new(
            id,
            title,
            InputMessageContent::Text(InputMessageContentText::new(body)),
        )
        .description(desc),
    )
}

fn truncate(s: &str, max: usize) -> String {
    let mut out: String = s.chars().take(max).collect();
    if s.chars().count() > max {
        out.push('…');
    }
    out
}
