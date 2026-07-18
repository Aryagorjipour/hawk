use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{
    CrawlJobId, CrawlSource, CrawlStatus, DomainError, DomainResult, HistoryEntry, HistoryEntryId,
    ScheduleId, UserId,
};
use crate::ports::HistoryRepository;

pub struct SqliteHistoryRepository {
    pool: SqlitePool,
}

impl SqliteHistoryRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl HistoryRepository for SqliteHistoryRepository {
    async fn insert(&self, entry: &HistoryEntry) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO history_entries (
                id, user_id, crawl_job_id, status, start_url, prompt_snippet,
                result_pretty, error_detail, source, occurred_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(entry.id.to_string())
        .bind(entry.user_id.to_string())
        .bind(entry.crawl_job_id.to_string())
        .bind(entry.status.as_str())
        .bind(&entry.start_url)
        .bind(&entry.prompt_snippet)
        .bind(&entry.result_pretty)
        .bind(&entry.error_detail)
        .bind(entry.source.as_str())
        .bind(entry.occurred_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<HistoryEntry>> {
        let rows = sqlx::query_as::<_, HistRow>(
            r#"SELECT * FROM history_entries
               WHERE user_id = ?
               ORDER BY occurred_at DESC
               LIMIT ? OFFSET ?"#,
        )
        .bind(user_id.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
        .map_err(persist)?;
        rows.into_iter().map(HistRow::into_entry).collect()
    }

    async fn get(&self, id: HistoryEntryId) -> DomainResult<Option<HistoryEntry>> {
        let row = sqlx::query_as::<_, HistRow>(r#"SELECT * FROM history_entries WHERE id = ?"#)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(persist)?;
        row.map(HistRow::into_entry).transpose()
    }

    async fn list_by_schedule(
        &self,
        user_id: UserId,
        schedule_id: ScheduleId,
        limit: i64,
    ) -> DomainResult<Vec<HistoryEntry>> {
        let rows = sqlx::query_as::<_, HistRow>(
            r#"
            SELECT h.* FROM history_entries h
            INNER JOIN crawl_jobs c ON c.id = h.crawl_job_id
            WHERE h.user_id = ? AND c.schedule_id = ?
            ORDER BY h.occurred_at DESC
            LIMIT ?
            "#,
        )
        .bind(user_id.to_string())
        .bind(schedule_id.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(persist)?;
        rows.into_iter().map(HistRow::into_entry).collect()
    }

    async fn trim_to_cap(&self, user_id: UserId, cap: i64) -> DomainResult<()> {
        sqlx::query(
            r#"
            DELETE FROM history_entries
            WHERE user_id = ?
              AND id NOT IN (
                SELECT id FROM history_entries
                WHERE user_id = ?
                ORDER BY occurred_at DESC
                LIMIT ?
              )
            "#,
        )
        .bind(user_id.to_string())
        .bind(user_id.to_string())
        .bind(cap)
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct HistRow {
    id: String,
    user_id: String,
    crawl_job_id: String,
    status: String,
    start_url: String,
    prompt_snippet: String,
    result_pretty: Option<String>,
    error_detail: Option<String>,
    source: String,
    occurred_at: String,
}

impl HistRow {
    fn into_entry(self) -> DomainResult<HistoryEntry> {
        Ok(HistoryEntry {
            id: HistoryEntryId::parse(&self.id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            user_id: UserId::parse(&self.user_id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            crawl_job_id: CrawlJobId::parse(&self.crawl_job_id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            status: CrawlStatus::parse(&self.status)?,
            start_url: self.start_url,
            prompt_snippet: self.prompt_snippet,
            result_pretty: self.result_pretty,
            error_detail: self.error_detail,
            source: CrawlSource::parse(&self.source)?,
            occurred_at: parse_dt(&self.occurred_at)?,
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
