use std::sync::Arc;

use teloxide::prelude::Requester;
use teloxide::Bot;
use tokio::sync::mpsc;

use super::pending_inline::PendingInlineStore;
use crate::adapters::crawler::HttpPageFetcher;
use crate::adapters::i18n::I18n;
use crate::application::{
    HistoryService, ManageScheduleService, OnboardService, PurchaseService, SettingsService,
    StartCrawlService,
};
use crate::bootstrap::Config;
use crate::domain::CrawlJobId;
use crate::infrastructure::EventBus;
use crate::ports::Mailer;

#[derive(Clone)]
pub struct AppState {
    pub bot: Bot,
    pub config: Arc<Config>,
    pub i18n: I18n,
    pub onboard: Arc<OnboardService>,
    pub crawls: Arc<StartCrawlService>,
    pub schedules: Arc<ManageScheduleService>,
    pub settings: Arc<SettingsService>,
    pub history: Arc<HistoryService>,
    pub purchases: Arc<PurchaseService>,
    pub mailer: Arc<dyn Mailer>,
    pub events: EventBus,
    pub crawl_tx: mpsc::Sender<CrawlJobId>,
    pub fetcher: Arc<HttpPageFetcher>,
    pub pending_inline: Arc<PendingInlineStore>,
}

impl AppState {
    pub async fn notify_chat(&self, telegram_user_id: i64, text: String) {
        let chat_id = teloxide::types::ChatId(telegram_user_id);
        if let Err(e) = send_long(&self.bot, chat_id, &text).await {
            tracing::warn!(error = %e, "notify_chat_failed");
        }
    }
}

pub async fn send_long(
    bot: &Bot,
    chat_id: teloxide::types::ChatId,
    text: &str,
) -> Result<(), teloxide::RequestError> {
    const LIMIT: usize = 4000;
    if text.chars().count() <= LIMIT {
        bot.send_message(chat_id, text).await?;
        return Ok(());
    }
    let mut rest = text;
    let mut part = 1;
    while !rest.is_empty() {
        let take = rest
            .char_indices()
            .nth(LIMIT)
            .map(|(i, _)| i)
            .unwrap_or(rest.len());
        let chunk = &rest[..take];
        let header = if part == 1 {
            String::new()
        } else {
            format!("…continued ({part})\n")
        };
        bot.send_message(chat_id, format!("{header}{chunk}"))
            .await?;
        rest = &rest[take..];
        part += 1;
    }
    Ok(())
}
