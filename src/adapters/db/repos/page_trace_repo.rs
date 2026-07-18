use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::domain::{CrawlJobId, DomainError, DomainResult};
use crate::ports::PageTraceRepository;

pub struct SqlitePageTraceRepository {
    pool: SqlitePool,
}

impl SqlitePageTraceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PageTraceRepository for SqlitePageTraceRepository {
    #[allow(clippy::too_many_arguments)]
    async fn insert(
        &self,
        crawl_id: CrawlJobId,
        url: &str,
        fetch_mode: &str,
        http_status: Option<i32>,
        ok: bool,
        error_detail: Option<&str>,
        at: DateTime<Utc>,
    ) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO crawl_page_traces (
                id, crawl_job_id, url, fetch_mode, http_status, ok, error_detail, fetched_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(crawl_id.to_string())
        .bind(url)
        .bind(fetch_mode)
        .bind(http_status)
        .bind(if ok { 1 } else { 0 })
        .bind(error_detail)
        .bind(at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Persistence(e.to_string()))?;
        Ok(())
    }
}
