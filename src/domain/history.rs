use chrono::{DateTime, Utc};

use super::crawl::{CrawlJob, CrawlSource, CrawlStatus};
use super::ids::{CrawlJobId, HistoryEntryId, UserId};

pub const HISTORY_CAP_PER_USER: i64 = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryEntry {
    pub id: HistoryEntryId,
    pub user_id: UserId,
    pub crawl_job_id: CrawlJobId,
    pub status: CrawlStatus,
    pub start_url: String,
    pub prompt_snippet: String,
    pub result_pretty: Option<String>,
    pub error_detail: Option<String>,
    pub source: CrawlSource,
    pub occurred_at: DateTime<Utc>,
}

impl HistoryEntry {
    pub fn from_finished_job(job: &CrawlJob, now: DateTime<Utc>) -> Self {
        let snippet = truncate(&job.user_prompt, 80);
        Self {
            id: HistoryEntryId::new(),
            user_id: job.user_id,
            crawl_job_id: job.id,
            status: job.status,
            start_url: job.start_url.clone(),
            prompt_snippet: snippet,
            result_pretty: job.result_pretty.clone(),
            error_detail: job.error_detail.clone(),
            source: job.source,
            occurred_at: job.finished_at.unwrap_or(now),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i >= max {
            out.push('…');
            break;
        }
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::crawl::CrawlJob;
    use chrono::TimeZone;

    #[test]
    fn builds_snippet() {
        let now = Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap();
        let mut job = CrawlJob::new(
            UserId::new(),
            CrawlSource::Interactive,
            None,
            "https://example.com".into(),
            "x".repeat(100),
            now,
        )
        .unwrap();
        job.mark_running(now).unwrap();
        job.mark_failed("x", "y", 0, now).unwrap();
        let h = HistoryEntry::from_finished_job(&job, now);
        assert!(h.prompt_snippet.ends_with('…'));
        assert!(h.prompt_snippet.chars().count() <= 81);
    }
}
