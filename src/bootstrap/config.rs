use std::env;
use std::path::{Path, PathBuf};

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
        load_dotenv_files();

        let telegram_bot_token = required("TELEGRAM_BOT_TOKEN")?;
        let master_key = required("SMART_HAWK_MASTER_KEY")?;
        let database_url = env_string("DATABASE_URL")
            .unwrap_or_else(|| "sqlite:data/smart-hawk.db?mode=rwc".into());
        let about_landing_url =
            env_string("ABOUT_LANDING_URL").unwrap_or_else(|| "https://smarthawk.dev".into());
        let about_github_url = env_string("ABOUT_GITHUB_URL")
            .unwrap_or_else(|| "https://github.com/Aryagorjipour/hawk".into());

        let smtp_url = env_opt("SMTP_URL");
        let smtp_from = env_opt("SMTP_FROM").or_else(|| env_opt("EMAIL_FROM"));
        let resend_api_key = env_opt("RESEND_API_KEY");
        // Accept RESEND_FROM, EMAIL_FROM, or SMTP_FROM (any non-empty)
        let resend_from = env_opt("RESEND_FROM")
            .or_else(|| env_opt("EMAIL_FROM"))
            .or_else(|| smtp_from.clone());

        let chromium_path = env_opt("CHROMIUM_PATH").map(PathBuf::from);

        let operator_notify_chat_id = env_opt("OPERATOR_NOTIFY_CHAT_ID")
            .map(|s| {
                s.parse::<i64>().map_err(|_| {
                    DomainError::Validation("OPERATOR_NOTIFY_CHAT_ID must be i64".into())
                })
            })
            .transpose()?;

        let worker_pool_size = env_opt("WORKER_POOL_SIZE")
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);

        let schedule_poll_secs = env_opt("SCHEDULE_POLL_SECS")
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

    /// Safe diagnostics (never logs secrets).
    pub fn email_diag(&self) -> String {
        format!(
            "resend_key={} resend_from={} smtp_url={} smtp_from={}",
            self.resend_api_key.is_some(),
            self.resend_from.is_some(),
            self.smtp_url.is_some(),
            self.smtp_from.is_some(),
        )
    }
}

/// Load `.env` from common locations. Non-empty file values **override empty** env vars.
fn load_dotenv_files() {
    let candidates = [
        PathBuf::from(".env"),
        PathBuf::from("/app/.env"),
        PathBuf::from("/data/.env"),
        env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join(".env")))
            .unwrap_or_default(),
    ];

    for path in &candidates {
        if path.as_os_str().is_empty() || !path.is_file() {
            continue;
        }
        apply_dotenv_file(path);
    }

    // Standard dotenv as well (does not override existing non-empty vars)
    let _ = dotenvy::dotenv();
}

/// Parse a dotenv-style file and set env vars. Non-empty file values win over empty process env.
fn apply_dotenv_file(path: &Path) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = strip_env_value(value);
        if value.is_empty() {
            continue;
        }
        // Override if missing or empty in the process environment
        match env::var(key) {
            Ok(existing) if !existing.trim().is_empty() => {}
            _ => env::set_var(key, &value),
        }
    }
}

/// Trim, strip matching quotes, keep content (supports `Name <email@x>`).
fn strip_env_value(raw: &str) -> String {
    let mut v = raw.trim().to_string();
    if v.len() >= 2 {
        let bytes = v.as_bytes();
        let q = bytes[0];
        if (q == b'"' || q == b'\'') && bytes[bytes.len() - 1] == q {
            v = v[1..v.len() - 1].to_string();
        }
    }
    // Remove accidental UTF-8 BOM / zero-width junk on values
    v.trim()
        .trim_start_matches('\u{feff}')
        .to_string()
}

fn env_string(key: &str) -> Option<String> {
    env_opt(key)
}

/// Read env var; treat whitespace-only as missing.
fn env_opt(key: &str) -> Option<String> {
    env::var(key).ok().and_then(|s| {
        let t = s.trim().to_string();
        if t.is_empty() {
            None
        } else {
            Some(t)
        }
    })
}

fn required(key: &str) -> DomainResult<String> {
    env_opt(key).ok_or_else(|| {
        DomainError::Validation(format!("missing required environment variable {key}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn strip_quoted_from_address() {
        assert_eq!(
            strip_env_value(r#""Smart Hawk <hawk@example.com>""#),
            "Smart Hawk <hawk@example.com>"
        );
        assert_eq!(
            strip_env_value("Smart Hawk <hawk@example.com>"),
            "Smart Hawk <hawk@example.com>"
        );
        assert_eq!(strip_env_value("hawk@example.com"), "hawk@example.com");
    }

    #[test]
    fn dotenv_file_overrides_empty_env() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "RESEND_FROM=Smart Hawk <hawk@example.com>").unwrap();
        writeln!(f, "RESEND_API_KEY=re_test").unwrap();

        env::set_var("RESEND_FROM", "");
        env::remove_var("RESEND_API_KEY");

        apply_dotenv_file(&path);

        assert_eq!(
            env::var("RESEND_FROM").unwrap(),
            "Smart Hawk <hawk@example.com>"
        );
        assert_eq!(env::var("RESEND_API_KEY").unwrap(), "re_test");

        env::remove_var("RESEND_FROM");
        env::remove_var("RESEND_API_KEY");
    }
}
