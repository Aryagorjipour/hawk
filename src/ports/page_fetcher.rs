use async_trait::async_trait;
use url::Url;

use crate::domain::{DomainResult, FetchMode};

#[derive(Debug, Clone)]
pub struct FetchedPage {
    pub url: String,
    pub final_url: String,
    pub status: u16,
    pub content_type: Option<String>,
    pub title: Option<String>,
    pub text: String,
    pub raw_excerpt: String,
    pub fetch_mode: FetchMode,
}

impl FetchedPage {
    pub fn is_usable(&self) -> bool {
        use crate::domain::MIN_USABLE_TEXT_LEN;
        if self.text.chars().count() >= MIN_USABLE_TEXT_LEN {
            return true;
        }
        // JSON/API tiny payloads still OK if parseable structure-ish
        let t = self.text.trim();
        (t.starts_with('{') || t.starts_with('[')) && t.len() >= 2
    }

    pub fn fingerprint_sample(&self) -> String {
        if let Some(t) = &self.title {
            if t.chars().count() >= 8 {
                return t.clone();
            }
        }
        self.text
            .chars()
            .filter(|c| !c.is_whitespace() || *c == ' ')
            .take(80)
            .collect::<String>()
            .trim()
            .to_string()
    }
}

#[async_trait]
pub trait PageFetcher: Send + Sync {
    /// HTTP fetch with SSRF checks on resolved IPs.
    async fn fetch_http(&self, url: &Url) -> DomainResult<FetchedPage>;

    /// Browser fallback; may return FetchFailed if browser disabled.
    async fn fetch_browser(&self, url: &Url) -> DomainResult<FetchedPage>;

    /// Prefer HTTP then browser.
    async fn fetch_resilient(&self, url: &Url) -> DomainResult<FetchedPage> {
        match self.fetch_http(url).await {
            Ok(page) if page.is_usable() => Ok(page),
            Ok(thin) => match self.fetch_browser(url).await {
                Ok(browser) if browser.is_usable() => Ok(browser),
                Ok(_) => Ok(thin),
                Err(_) => Ok(thin),
            },
            Err(http_err) => match self.fetch_browser(url).await {
                Ok(page) => Ok(page),
                Err(_) => Err(http_err),
            },
        }
    }
}
