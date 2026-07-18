use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{
    CrawlJob, CrawlJobId, CrawlSource, CrawlStatus, DomainError, DomainResult, ScheduleId,
    StructuredCrawlResult, UserId,
};
use crate::ports::CrawlRepository;

pub struct SqliteCrawlRepository {
    pool: SqlitePool,
}

impl SqliteCrawlRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CrawlRepository for SqliteCrawlRepository {
    async fn insert(&self, job: &CrawlJob) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO crawl_jobs (
                id, user_id, source, schedule_id, start_url, user_prompt, status,
                pages_fetched, result_json, result_pretty, error_kind, error_detail,
                created_at, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(job.id.to_string())
        .bind(job.user_id.to_string())
        .bind(job.source.as_str())
        .bind(job.schedule_id.map(|s| s.to_string()))
        .bind(&job.start_url)
        .bind(&job.user_prompt)
        .bind(job.status.as_str())
        .bind(job.pages_fetched as i64)
        .bind(
            job.result
                .as_ref()
                .and_then(|r| serde_json::to_string(r).ok()),
        )
        .bind(&job.result_pretty)
        .bind(&job.error_kind)
        .bind(&job.error_detail)
        .bind(job.created_at.to_rfc3339())
        .bind(job.started_at.map(|t| t.to_rfc3339()))
        .bind(job.finished_at.map(|t| t.to_rfc3339()))
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn update(&self, job: &CrawlJob) -> DomainResult<()> {
        let res = sqlx::query(
            r#"
            UPDATE crawl_jobs SET
                status = ?, pages_fetched = ?, result_json = ?, result_pretty = ?,
                error_kind = ?, error_detail = ?, started_at = ?, finished_at = ?
            WHERE id = ?
            "#,
        )
        .bind(job.status.as_str())
        .bind(job.pages_fetched as i64)
        .bind(
            job.result
                .as_ref()
                .and_then(|r| serde_json::to_string(r).ok()),
        )
        .bind(&job.result_pretty)
        .bind(&job.error_kind)
        .bind(&job.error_detail)
        .bind(job.started_at.map(|t| t.to_rfc3339()))
        .bind(job.finished_at.map(|t| t.to_rfc3339()))
        .bind(job.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound("crawl job".into()));
        }
        Ok(())
    }

    async fn get(&self, id: CrawlJobId) -> DomainResult<Option<CrawlJob>> {
        let row = sqlx::query_as::<_, CrawlRow>(r#"SELECT * FROM crawl_jobs WHERE id = ?"#)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(persist)?;
        row.map(CrawlRow::into_job).transpose()
    }

    async fn list_by_user(&self, user_id: UserId, limit: i64) -> DomainResult<Vec<CrawlJob>> {
        let rows = sqlx::query_as::<_, CrawlRow>(
            r#"SELECT * FROM crawl_jobs WHERE user_id = ? ORDER BY created_at DESC LIMIT ?"#,
        )
        .bind(user_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(persist)?;
        rows.into_iter().map(CrawlRow::into_job).collect()
    }

    async fn has_active_for_user(&self, user_id: UserId) -> DomainResult<bool> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM crawl_jobs
               WHERE user_id = ? AND status IN ('queued', 'running')"#,
        )
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(persist)?;
        Ok(count > 0)
    }

    async fn claim_next_queued(&self) -> DomainResult<Option<CrawlJob>> {
        let mut tx = self.pool.begin().await.map_err(persist)?;
        let row = sqlx::query_as::<_, CrawlRow>(
            r#"SELECT * FROM crawl_jobs WHERE status = 'queued'
               ORDER BY created_at ASC LIMIT 1"#,
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(persist)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let mut job = row.into_job()?;
        let now = Utc::now();
        job.mark_running(now)?;
        sqlx::query(
            r#"UPDATE crawl_jobs SET status = 'running', started_at = ? WHERE id = ? AND status = 'queued'"#,
        )
        .bind(now.to_rfc3339())
        .bind(job.id.to_string())
        .execute(&mut *tx)
        .await
        .map_err(persist)?;
        tx.commit().await.map_err(persist)?;
        Ok(Some(job))
    }

    async fn count_by_user_status(
        &self,
        user_id: UserId,
        status: CrawlStatus,
    ) -> DomainResult<i64> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM crawl_jobs WHERE user_id = ? AND status = ?"#,
        )
        .bind(user_id.to_string())
        .bind(status.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(persist)?;
        Ok(count)
    }
}

#[derive(sqlx::FromRow)]
struct CrawlRow {
    id: String,
    user_id: String,
    source: String,
    schedule_id: Option<String>,
    start_url: String,
    user_prompt: String,
    status: String,
    pages_fetched: i64,
    result_json: Option<String>,
    result_pretty: Option<String>,
    error_kind: Option<String>,
    error_detail: Option<String>,
    created_at: String,
    started_at: Option<String>,
    finished_at: Option<String>,
}

impl CrawlRow {
    fn into_job(self) -> DomainResult<CrawlJob> {
        let result = match self.result_json {
            Some(j) => Some(
                serde_json::from_str::<StructuredCrawlResult>(&j)
                    .map_err(|e| DomainError::Persistence(e.to_string()))?,
            ),
            None => None,
        };
        Ok(CrawlJob {
            id: CrawlJobId::parse(&self.id).map_err(|e| DomainError::Persistence(e.to_string()))?,
            user_id: UserId::parse(&self.user_id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            source: CrawlSource::parse(&self.source)?,
            schedule_id: self
                .schedule_id
                .as_deref()
                .map(ScheduleId::parse)
                .transpose()
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            start_url: self.start_url,
            user_prompt: self.user_prompt,
            status: CrawlStatus::parse(&self.status)?,
            pages_fetched: self.pages_fetched as u8,
            result,
            result_pretty: self.result_pretty,
            error_kind: self.error_kind,
            error_detail: self.error_detail,
            created_at: parse_dt(&self.created_at)?,
            started_at: self.started_at.as_deref().map(parse_dt).transpose()?,
            finished_at: self.finished_at.as_deref().map(parse_dt).transpose()?,
        })
    }
}

fn parse_dt(s: &str) -> DomainResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| DomainError::Persistence(e.to_string()))
}

fn persist(e: sqlx::Error) -> DomainError {
    DomainError::Persistence(e.to_string())
}
