use std::net::ToSocketAddrs;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::{redirect::Policy, Client};
use url::Url;

use super::extract::extract_from_html;
use crate::domain::{assert_public_ip, DomainError, DomainResult, FetchMode};
use crate::ports::{FetchedPage, PageFetcher};

pub struct HttpPageFetcher {
    client: Client,
    browser: Option<BrowserFallback>,
    accept_language: String,
}

#[derive(Clone)]
pub struct BrowserFallback {
    pub enabled: bool,
    pub chromium_path: Option<std::path::PathBuf>,
}

impl HttpPageFetcher {
    pub fn new(browser: BrowserFallback, accept_language: impl Into<String>) -> DomainResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(20))
            .connect_timeout(Duration::from_secs(10))
            .redirect(Policy::limited(5))
            .user_agent(concat!(
                "Mozilla/5.0 (compatible; SmartHawk/0.1; ",
                "+https://smarthawk.dev; personal crawler)"
            ))
            .gzip(true)
            .build()
            .map_err(|e| DomainError::FetchFailed(e.to_string()))?;
        Ok(Self {
            client,
            browser: Some(browser),
            accept_language: accept_language.into(),
        })
    }

    fn resolve_and_check_ssrf(&self, url: &Url) -> DomainResult<()> {
        let host = url
            .host_str()
            .ok_or_else(|| DomainError::InvalidUrl("missing host".into()))?;

        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            assert_public_ip(ip)?;
            return Ok(());
        }

        let port = url.port_or_known_default().unwrap_or(80);
        let addrs = (host, port)
            .to_socket_addrs()
            .map_err(|e| DomainError::FetchFailed(format!("DNS failed for {host}: {e}")))?;

        let mut any = false;
        for addr in addrs {
            any = true;
            assert_public_ip(addr.ip())?;
        }
        if !any {
            return Err(DomainError::FetchFailed(format!(
                "no addresses for host {host}"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl PageFetcher for HttpPageFetcher {
    async fn fetch_http(&self, url: &Url) -> DomainResult<FetchedPage> {
        self.resolve_and_check_ssrf(url)?;

        let response = self
            .client
            .get(url.clone())
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/json;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", &self.accept_language)
            .send()
            .await
            .map_err(|e| DomainError::FetchFailed(e.to_string()))?;

        // Re-check final URL after redirects
        let final_url = response.url().clone();
        self.resolve_and_check_ssrf(&final_url)?;

        let status = response.status().as_u16();
        if status == 401 || status == 403 {
            return Err(DomainError::FetchFailed(format!(
                "access denied (HTTP {status}) — the site waved us off"
            )));
        }
        if status == 404 {
            return Err(DomainError::FetchFailed("page not found (HTTP 404)".into()));
        }
        if status == 429 {
            return Err(DomainError::FetchFailed(
                "rate limited (HTTP 429) — try later".into(),
            ));
        }
        if status >= 400 {
            return Err(DomainError::FetchFailed(format!(
                "unexpected HTTP status {status}"
            )));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = response
            .bytes()
            .await
            .map_err(|e| DomainError::FetchFailed(e.to_string()))?;

        if bytes.len() > 5 * 1024 * 1024 {
            return Err(DomainError::FetchFailed(
                "response body exceeds 5 MB limit".into(),
            ));
        }

        let body = String::from_utf8_lossy(&bytes).into_owned();
        let extracted = extract_from_html(&body, content_type.as_deref());

        Ok(FetchedPage {
            url: url.to_string(),
            final_url: final_url.to_string(),
            status,
            content_type,
            title: extracted.title,
            text: extracted.text,
            raw_excerpt: extracted.raw_excerpt,
            fetch_mode: FetchMode::Http,
        })
    }

    async fn fetch_browser(&self, url: &Url) -> DomainResult<FetchedPage> {
        let Some(cfg) = &self.browser else {
            return Err(DomainError::FetchFailed(
                "browser fallback not configured".into(),
            ));
        };
        if !cfg.enabled {
            return Err(DomainError::FetchFailed(
                "browser fallback disabled (set CHROMIUM_PATH to enable)".into(),
            ));
        }
        self.resolve_and_check_ssrf(url)?;

        // Lightweight headless via chromium CLI if available: dump DOM with --dump-dom
        // Avoid hard dep on chromiumoxide for portability; shell out.
        let chrome = cfg
            .chromium_path
            .clone()
            .or_else(find_chromium)
            .ok_or_else(|| {
                DomainError::FetchFailed("no Chromium binary found for browser fallback".into())
            })?;

        let output = tokio::process::Command::new(chrome)
            .arg("--headless=new")
            .arg("--disable-gpu")
            .arg("--no-sandbox")
            .arg("--virtual-time-budget=10000")
            .arg("--dump-dom")
            .arg(url.as_str())
            .output()
            .await
            .map_err(|e| DomainError::FetchFailed(format!("chromium spawn failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DomainError::FetchFailed(format!(
                "chromium failed: {stderr}"
            )));
        }

        let body = String::from_utf8_lossy(&output.stdout).into_owned();
        if body.trim().is_empty() {
            return Err(DomainError::FetchFailed(
                "chromium returned empty DOM".into(),
            ));
        }

        let extracted = extract_from_html(&body, Some("text/html"));
        Ok(FetchedPage {
            url: url.to_string(),
            final_url: url.to_string(),
            status: 200,
            content_type: Some("text/html".into()),
            title: extracted.title,
            text: extracted.text,
            raw_excerpt: extracted.raw_excerpt,
            fetch_mode: FetchMode::Browser,
        })
    }
}

fn find_chromium() -> Option<std::path::PathBuf> {
    const CANDIDATES: &[&str] = &[
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
        "chrome",
    ];
    for name in CANDIDATES {
        if let Ok(path) = which_bin(name) {
            return Some(path);
        }
    }
    None
}

fn which_bin(name: &str) -> Result<std::path::PathBuf, ()> {
    let output = std::process::Command::new("which")
        .arg(name)
        .output()
        .map_err(|_| ())?;
    if !output.status.success() {
        return Err(());
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return Err(());
    }
    Ok(std::path::PathBuf::from(path))
}
