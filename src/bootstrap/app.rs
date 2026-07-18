use std::sync::Arc;
use std::time::Duration;

use teloxide::prelude::*;
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::adapters::clock::SystemClock;
use crate::adapters::crawler::{BrowserFallback, HttpPageFetcher};
use crate::adapters::crypto::AesGcmSecretBox;
use crate::adapters::db::{
    connect, SqliteConversationRepository, SqliteCrawlRepository, SqliteHistoryRepository,
    SqlitePageTraceRepository, SqlitePaymentRepository, SqliteScheduleRepository,
    SqliteUserRepository,
};
use crate::adapters::email::build_mailer;
use crate::adapters::i18n::I18n;
use crate::adapters::telegram::{run_bot, AppState};
use crate::application::{
    HistoryService, ManageScheduleService, OnboardService, PurchaseService, RunDueSchedulesService,
    SettingsService, StartCrawlService,
};
use crate::bootstrap::Config;
use crate::domain::{CrawlJobId, DomainError, DomainResult};
use crate::infrastructure::{tracing_handler, EventBus};
use crate::ports::{Clock, UserRepository};

pub async fn run(config: Config) -> DomainResult<()> {
    let pool = connect(&config.database_url).await?;
    let secrets = Arc::new(AesGcmSecretBox::from_master_key_str(&config.master_key)?);
    let clock: Arc<dyn Clock> = Arc::new(SystemClock);
    let events = EventBus::new();
    events.subscribe(tracing_handler());

    let users = Arc::new(SqliteUserRepository::new(pool.clone()));
    let conversations = Arc::new(SqliteConversationRepository::new(pool.clone()));
    let crawls_repo = Arc::new(SqliteCrawlRepository::new(pool.clone()));
    let schedules_repo = Arc::new(SqliteScheduleRepository::new(pool.clone()));
    let history_repo = Arc::new(SqliteHistoryRepository::new(pool.clone()));
    let payments_repo = Arc::new(SqlitePaymentRepository::new(pool.clone()));
    let traces_repo = Arc::new(SqlitePageTraceRepository::new(pool.clone()));

    let browser = BrowserFallback {
        enabled: config.chromium_path.is_some()
            || std::process::Command::new("which")
                .arg("chromium")
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        chromium_path: config.chromium_path.clone(),
    };
    let fetcher = Arc::new(HttpPageFetcher::new(browser, "en-US,en;q=0.9")?);

    let mailer = build_mailer(&config)?;

    let onboard = Arc::new(OnboardService {
        users: users.clone(),
        conversations: conversations.clone(),
        secrets: secrets.clone(),
        clock: clock.clone(),
        events: events.clone(),
    });

    let crawls = Arc::new(StartCrawlService {
        users: users.clone(),
        crawls: crawls_repo.clone(),
        history: history_repo.clone(),
        traces: traces_repo.clone(),
        fetcher: fetcher.clone(),
        secrets: secrets.clone(),
        clock: clock.clone(),
        events: events.clone(),
    });

    let schedules = Arc::new(ManageScheduleService {
        users: users.clone(),
        schedules: schedules_repo.clone(),
        clock: clock.clone(),
    });

    let settings = Arc::new(SettingsService {
        users: users.clone(),
        conversations: conversations.clone(),
        clock: clock.clone(),
        events: events.clone(),
    });

    let history = Arc::new(HistoryService {
        history: history_repo.clone(),
    });

    let purchases = Arc::new(PurchaseService {
        users: users.clone(),
        payments: payments_repo.clone(),
        clock: clock.clone(),
        events: events.clone(),
    });

    let bot = Bot::new(config.telegram_bot_token.clone());
    let (crawl_tx, mut crawl_rx) = mpsc::channel::<CrawlJobId>(64);

    let config = Arc::new(config);
    let i18n = I18n::load_embedded();

    let state = AppState {
        bot: bot.clone(),
        config: config.clone(),
        i18n,
        onboard: onboard.clone(),
        crawls: crawls.clone(),
        schedules: schedules.clone(),
        settings,
        history,
        purchases,
        mailer: mailer.clone(),
        events: events.clone(),
        crawl_tx: crawl_tx.clone(),
        fetcher: fetcher.clone(),
        pending_inline: Arc::new(crate::adapters::telegram::PendingInlineStore::new()),
    };

    // Crawl worker pool
    let worker_crawls = crawls.clone();
    let worker_bot = bot.clone();
    let worker_users = users.clone();
    let worker_i18n = state.i18n.clone();
    let pool_size = config.worker_pool_size;
    let sem = Arc::new(tokio::sync::Semaphore::new(pool_size));

    tokio::spawn(async move {
        while let Some(job_id) = crawl_rx.recv().await {
            let permit = match sem.clone().acquire_owned().await {
                Ok(p) => p,
                Err(_) => break,
            };
            let worker_crawls = worker_crawls.clone();
            let worker_bot = worker_bot.clone();
            let worker_users = worker_users.clone();
            let worker_i18n = worker_i18n.clone();
            tokio::spawn(async move {
                let _permit = permit;
                let job = match worker_crawls.crawls.get(job_id).await {
                    Ok(Some(j)) => j,
                    Ok(None) => return,
                    Err(e) => {
                        error!(error = %e, "load_job_failed");
                        return;
                    }
                };
                let user_id = job.user_id;
                let finished = match worker_crawls.execute_job(job).await {
                    Ok(j) => j,
                    Err(e) => {
                        error!(error = %e, %job_id, "execute_job_failed");
                        return;
                    }
                };
                if let Ok(Some(user)) = worker_users.get_by_id(user_id).await {
                    let text = if let Some(pretty) = &finished.result_pretty {
                        let budget = user.credits.total_crawl_budget_hint(chrono::Utc::now());
                        format!(
                            "{pretty}\n\n{}",
                            worker_i18n.t(
                                user.locale,
                                "crawl-done-footer",
                                &[
                                    ("pages", finished.pages_fetched.to_string()),
                                    ("budget", budget.to_string()),
                                ],
                            )
                        )
                    } else {
                        format_crawl_failure(&worker_i18n, user.locale, &finished)
                    };
                    let chat = teloxide::types::ChatId(user.telegram_user_id.get());
                    if let Err(e) =
                        crate::adapters::telegram::state::send_long(&worker_bot, chat, &text).await
                    {
                        error!(error = %e, "deliver_crawl_result_failed");
                    }
                }
            });
        }
    });

    // Schedule poller
    let due_crawls = crawls.clone();
    let due_schedules = schedules_repo.clone();
    let due_users = users.clone();
    let due_mailer = mailer.clone();
    let due_clock = clock.clone();
    let due_events = events.clone();
    let due_bot = bot.clone();
    let poll_secs = config.schedule_poll_secs;

    tokio::spawn(async move {
        let chat_notify: Arc<
            dyn Fn(i64, String) -> futures::future::BoxFuture<'static, ()> + Send + Sync,
        > = Arc::new({
            let bot = due_bot.clone();
            move |tg_id: i64, text: String| {
                let bot = bot.clone();
                Box::pin(async move {
                    let chat = teloxide::types::ChatId(tg_id);
                    let _ = crate::adapters::telegram::state::send_long(&bot, chat, &text).await;
                })
            }
        });

        let runner = RunDueSchedulesService {
            users: due_users,
            schedules: due_schedules,
            crawls: due_crawls,
            mailer: due_mailer,
            clock: due_clock,
            events: due_events,
            chat_notify,
        };

        loop {
            if let Err(e) = runner.tick(20).await {
                error!(error = %e, "schedule_tick_failed");
            }
            tokio::time::sleep(Duration::from_secs(poll_secs)).await;
        }
    });

    info!("starting telegram bot");
    run_bot(state)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;
    Ok(())
}

fn format_crawl_failure(
    i18n: &I18n,
    locale: crate::domain::Locale,
    job: &crate::domain::CrawlJob,
) -> String {
    let detail = job.error_detail.clone().unwrap_or_else(|| "unknown".into());
    let key = match job.error_kind.as_deref() {
        Some("llm_auth") => "error-llm-auth",
        Some("llm_quota") => "error-llm-quota",
        Some("llm_rate") => "error-llm-rate",
        Some("llm_model") => "error-llm-model",
        Some("llm_network") => "error-llm-network",
        Some("llm_bad_response") => "error-llm-bad-response",
        Some("llm_unknown") | Some("llm") => "error-llm-unknown",
        Some("fetch") => "error-fetch",
        Some("unable") => "crawl-unable",
        _ => "crawl-failed",
    };
    if key == "crawl-failed" {
        i18n.t(locale, key, &[("detail", detail)])
    } else if key == "crawl-unable" {
        i18n.t(locale, key, &[("reason", detail)])
    } else if key == "error-fetch" {
        i18n.t(locale, key, &[("detail", detail)])
    } else {
        // LLM-specific: friendly fixed copy (detail already user-facing if stored)
        let friendly = i18n.t0(locale, key);
        if detail.is_empty() || detail == friendly {
            format!("🦅 {friendly}")
        } else {
            // Prefer i18n fixed text; drop technical leftovers
            format!("🦅 {friendly}")
        }
    }
}
