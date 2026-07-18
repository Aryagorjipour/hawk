use std::env;
use std::path::PathBuf;

use crate::domain::{DomainError, DomainResult};

#[derive(Debug, Clone)]
pub struct Config {
    pub telegram_bot_token: String,
    pub master_key: String,
    pub database_url: String,
    pub about_landing_url: String,
    pub about_github_url: String,
    pub smtp_url: Option<String>,
    pub smtp_from: Option<String>,
    /// Prefer Resend when set; SMTP used as fallback.
    pub resend_api_key: Option<String>,
    pub resend_from: Option<String>,
    pub chromium_path: Option<PathBuf>,
    pub operator_notify_chat_id: Option<i64>,
    pub worker_pool_size: usize,
    pub schedule_poll_secs: u64,
}

impl Config {
    pub fn from_env() -> DomainResult<Self> {
        let _ = dotenvy::dotenv();

        let telegram_bot_token = required("TELEGRAM_BOT_TOKEN")?;
        let master_key = required("SMART_HAWK_MASTER_KEY")?;
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "sqlite:data/smart-hawk.db?mode=rwc".into());
        let about_landing_url =
            env::var("ABOUT_LANDING_URL").unwrap_or_else(|_| "https://smarthawk.dev".into());
        let about_github_url = env::var("ABOUT_GITHUB_URL")
            .unwrap_or_else(|_| "https://github.com/Aryagorjipour/hawk".into());

        let smtp_url = env::var("SMTP_URL").ok().filter(|s| !s.is_empty());
        let smtp_from = env::var("SMTP_FROM")
            .ok()
            .or_else(|| env::var("EMAIL_FROM").ok())
            .filter(|s| !s.is_empty());
        let resend_api_key = env::var("RESEND_API_KEY").ok().filter(|s| !s.is_empty());
        let resend_from = env::var("RESEND_FROM")
            .ok()
            .or_else(|| env::var("EMAIL_FROM").ok())
            .or_else(|| smtp_from.clone())
            .filter(|s| !s.is_empty());
        let chromium_path = env::var("CHROMIUM_PATH")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from);

        let operator_notify_chat_id = env::var("OPERATOR_NOTIFY_CHAT_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse::<i64>().map_err(|_| {
                    DomainError::Validation("OPERATOR_NOTIFY_CHAT_ID must be i64".into())
                })
            })
            .transpose()?;

        let worker_pool_size = env::var("WORKER_POOL_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        let schedule_poll_secs = env::var("SCHEDULE_POLL_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);

        Ok(Self {
            telegram_bot_token,
            master_key,
            database_url,
            about_landing_url,
            about_github_url,
            smtp_url,
            smtp_from,
            resend_api_key,
            resend_from,
            chromium_path,
            operator_notify_chat_id,
            worker_pool_size,
            schedule_poll_secs,
        })
    }
}

fn required(key: &str) -> DomainResult<String> {
    env::var(key).map_err(|_| {
        DomainError::Validation(format!("missing required environment variable {key}"))
    })
}
