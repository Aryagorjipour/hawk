use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};
use super::ids::{CrawlJobId, ScheduleId, UserId};

pub const MAX_PAGES_PER_CRAWL: u8 = 4;
pub const MIN_USABLE_TEXT_LEN: usize = 80;
pub const MAX_PAGE_CONTEXT_CHARS: usize = 200_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlSource {
    Interactive,
    Schedule,
    Inline,
}

impl CrawlSource {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::Schedule => "schedule",
            Self::Inline => "inline",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "interactive" => Ok(Self::Interactive),
            "schedule" => Ok(Self::Schedule),
            "inline" => Ok(Self::Inline),
            other => Err(DomainError::Parse(format!("unknown crawl source: {other}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl CrawlStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(DomainError::Parse(format!("unknown crawl status: {other}"))),
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FetchMode {
    Http,
    Browser,
}

impl FetchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http",
            Self::Browser => "browser",
        }
    }

    pub fn parse(s: &str) -> DomainResult<Self> {
        match s {
            "http" => Ok(Self::Http),
            "browser" => Ok(Self::Browser),
            other => Err(DomainError::Parse(format!("unknown fetch mode: {other}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlItem {
    pub label: String,
    pub value: String,
    #[serde(default)]
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrawlSourceRef {
    pub url: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractStatus {
    Ok,
    Partial,
    Unable,
}

impl ExtractStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Partial => "partial",
            Self::Unable => "unable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructuredCrawlResult {
    pub status: ExtractStatus,
    pub language: String,
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub items: Vec<CrawlItem>,
    #[serde(default)]
    pub sources: Vec<CrawlSourceRef>,
    #[serde(default)]
    pub follow_up_urls: Vec<String>,
    #[serde(default)]
    pub unable_reason: Option<String>,
}

impl StructuredCrawlResult {
    pub fn is_usable(&self) -> bool {
        !matches!(self.status, ExtractStatus::Unable)
            && (!self.summary.trim().is_empty() || !self.items.is_empty())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJob {
    pub id: CrawlJobId,
    pub user_id: UserId,
    pub source: CrawlSource,
    pub schedule_id: Option<ScheduleId>,
    pub start_url: String,
    pub user_prompt: String,
    pub status: CrawlStatus,
    pub pages_fetched: u8,
    pub result: Option<StructuredCrawlResult>,
    pub result_pretty: Option<String>,
    pub error_kind: Option<String>,
    pub error_detail: Option<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl CrawlJob {
    pub fn new(
        user_id: UserId,
        source: CrawlSource,
        schedule_id: Option<ScheduleId>,
        start_url: String,
        user_prompt: String,
        now: DateTime<Utc>,
    ) -> DomainResult<Self> {
        if start_url.trim().is_empty() {
            return Err(DomainError::Validation("start URL required".into()));
        }
        if user_prompt.trim().is_empty() {
            return Err(DomainError::Validation("prompt required".into()));
        }
        if matches!(source, CrawlSource::Schedule) && schedule_id.is_none() {
            return Err(DomainError::Validation(
                "schedule source requires schedule_id".into(),
            ));
        }
        Ok(Self {
            id: CrawlJobId::new(),
            user_id,
            source,
            schedule_id,
            start_url: start_url.trim().to_string(),
            user_prompt: user_prompt.trim().to_string(),
            status: CrawlStatus::Queued,
            pages_fetched: 0,
            result: None,
            result_pretty: None,
            error_kind: None,
            error_detail: None,
            created_at: now,
            started_at: None,
            finished_at: None,
        })
    }

    pub fn mark_running(&mut self, now: DateTime<Utc>) -> DomainResult<()> {
        if self.status != CrawlStatus::Queued {
            return Err(DomainError::Conflict(format!(
                "cannot run job in status {}",
                self.status.as_str()
            )));
        }
        self.status = CrawlStatus::Running;
        self.started_at = Some(now);
        Ok(())
    }

    pub fn mark_succeeded(
        &mut self,
        result: StructuredCrawlResult,
        pretty: String,
        pages_fetched: u8,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        if self.status != CrawlStatus::Running {
            return Err(DomainError::Conflict(
                "only running jobs can succeed".into(),
            ));
        }
        self.status = CrawlStatus::Succeeded;
        self.result = Some(result);
        self.result_pretty = Some(pretty);
        self.pages_fetched = pages_fetched;
        self.finished_at = Some(now);
        Ok(())
    }

    pub fn mark_failed(
        &mut self,
        kind: impl Into<String>,
        detail: impl Into<String>,
        pages_fetched: u8,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        if self.status != CrawlStatus::Running && self.status != CrawlStatus::Queued {
            return Err(DomainError::Conflict("job already terminal".into()));
        }
        self.status = CrawlStatus::Failed;
        self.error_kind = Some(kind.into());
        self.error_detail = Some(detail.into());
        self.pages_fetched = pages_fetched;
        self.finished_at = Some(now);
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PageBudget {
    remaining: u8,
    used: u8,
}

impl PageBudget {
    pub fn new(max: u8) -> Self {
        Self {
            remaining: max,
            used: 0,
        }
    }

    pub fn default_crawl() -> Self {
        Self::new(MAX_PAGES_PER_CRAWL)
    }

    pub fn try_consume(&mut self) -> bool {
        if self.remaining == 0 {
            return false;
        }
        self.remaining -= 1;
        self.used += 1;
        true
    }

    pub fn used(&self) -> u8 {
        self.used
    }

    pub fn remaining(&self) -> u8 {
        self.remaining
    }
}

/// Detect refusal-style LLM answers in free text (fallback heuristic).
pub fn looks_like_refusal(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    const PHRASES: &[&str] = &[
        "i couldn't",
        "i could not",
        "i can't",
        "i cannot",
        "unable to access",
        "unable to retrieve",
        "don't have access",
        "do not have access",
        "i'm unable",
        "i am unable",
        "cannot fetch",
        "can't fetch",
        "no access to the",
        "as an ai",
    ];
    PHRASES.iter().any(|p| lower.contains(p))
}

pub fn prettify_result(result: &StructuredCrawlResult) -> String {
    let mut out = String::new();
    if !result.title.trim().is_empty() {
        out.push_str(&format!("*{}*\n\n", escape_md(result.title.trim())));
    }
    if !result.summary.trim().is_empty() {
        out.push_str(result.summary.trim());
        out.push_str("\n\n");
    }
    if !result.items.is_empty() {
        for item in &result.items {
            out.push_str(&format!("• *{}*: {}\n", escape_md(&item.label), item.value));
            if let Some(ev) = &item.evidence {
                if !ev.trim().is_empty() {
                    out.push_str(&format!("  _{}_\n", escape_md(ev.trim())));
                }
            }
        }
        out.push('\n');
    }
    if !result.sources.is_empty() {
        out.push_str("Sources:\n");
        for s in &result.sources {
            if s.note.is_empty() {
                out.push_str(&format!("• {}\n", s.url));
            } else {
                out.push_str(&format!("• {} — {}\n", s.url, s.note));
            }
        }
    }
    out.trim().to_string()
}

fn escape_md(s: &str) -> String {
    s.replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('[', "\\[")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::UserId;
    use chrono::TimeZone;

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap()
    }

    #[test]
    fn page_budget_caps() {
        let mut b = PageBudget::new(2);
        assert!(b.try_consume());
        assert!(b.try_consume());
        assert!(!b.try_consume());
        assert_eq!(b.used(), 2);
    }

    #[test]
    fn refusal_detection() {
        assert!(looks_like_refusal("I couldn't access that page."));
        assert!(!looks_like_refusal("The price is $12."));
    }

    #[test]
    fn job_lifecycle() {
        let mut job = CrawlJob::new(
            UserId::new(),
            CrawlSource::Interactive,
            None,
            "https://example.com".into(),
            "get title".into(),
            now(),
        )
        .unwrap();
        job.mark_running(now()).unwrap();
        let result = StructuredCrawlResult {
            status: ExtractStatus::Ok,
            language: "en".into(),
            title: "Example".into(),
            summary: "A domain".into(),
            items: vec![],
            sources: vec![],
            follow_up_urls: vec![],
            unable_reason: None,
        };
        let pretty = prettify_result(&result);
        job.mark_succeeded(result, pretty, 1, now()).unwrap();
        assert_eq!(job.status, CrawlStatus::Succeeded);
    }
}
