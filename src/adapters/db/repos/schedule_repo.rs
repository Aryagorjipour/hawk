use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{
    CrawlJobId, DeliveryFlags, DomainError, DomainResult, Recurrence, Schedule, ScheduleId, UserId,
};
use crate::ports::ScheduleRepository;

pub struct SqliteScheduleRepository {
    pool: SqlitePool,
}

impl SqliteScheduleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ScheduleRepository for SqliteScheduleRepository {
    async fn insert(&self, schedule: &Schedule) -> DomainResult<()> {
        let rec = serde_json::to_string(&schedule.recurrence)
            .map_err(|e| DomainError::Persistence(e.to_string()))?;
        sqlx::query(
            r#"
            INSERT INTO schedules (
                id, user_id, label, start_url, user_prompt, recurrence_json, active,
                send_chat, send_email, send_trigger_msg, next_run_at, last_run_at,
                last_crawl_id, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(schedule.id.to_string())
        .bind(schedule.user_id.to_string())
        .bind(&schedule.label)
        .bind(&schedule.start_url)
        .bind(&schedule.user_prompt)
        .bind(rec)
        .bind(if schedule.active { 1 } else { 0 })
        .bind(if schedule.delivery.send_chat { 1 } else { 0 })
        .bind(if schedule.delivery.send_email { 1 } else { 0 })
        .bind(if schedule.delivery.send_trigger_message {
            1
        } else {
            0
        })
        .bind(schedule.next_run_at.to_rfc3339())
        .bind(schedule.last_run_at.map(|t| t.to_rfc3339()))
        .bind(schedule.last_crawl_id.map(|id| id.to_string()))
        .bind(schedule.created_at.to_rfc3339())
        .bind(schedule.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn update(&self, schedule: &Schedule) -> DomainResult<()> {
        let rec = serde_json::to_string(&schedule.recurrence)
            .map_err(|e| DomainError::Persistence(e.to_string()))?;
        let res = sqlx::query(
            r#"
            UPDATE schedules SET
                label = ?, start_url = ?, user_prompt = ?, recurrence_json = ?,
                active = ?, send_chat = ?, send_email = ?, send_trigger_msg = ?,
                next_run_at = ?, last_run_at = ?, last_crawl_id = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&schedule.label)
        .bind(&schedule.start_url)
        .bind(&schedule.user_prompt)
        .bind(rec)
        .bind(if schedule.active { 1 } else { 0 })
        .bind(if schedule.delivery.send_chat { 1 } else { 0 })
        .bind(if schedule.delivery.send_email { 1 } else { 0 })
        .bind(if schedule.delivery.send_trigger_message {
            1
        } else {
            0
        })
        .bind(schedule.next_run_at.to_rfc3339())
        .bind(schedule.last_run_at.map(|t| t.to_rfc3339()))
        .bind(schedule.last_crawl_id.map(|id| id.to_string()))
        .bind(schedule.updated_at.to_rfc3339())
        .bind(schedule.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound("schedule".into()));
        }
        Ok(())
    }

    async fn get(&self, id: ScheduleId) -> DomainResult<Option<Schedule>> {
        let row = sqlx::query_as::<_, SchedRow>(r#"SELECT * FROM schedules WHERE id = ?"#)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(persist)?;
        row.map(SchedRow::into_schedule).transpose()
    }

    async fn list_by_user(&self, user_id: UserId) -> DomainResult<Vec<Schedule>> {
        let rows = sqlx::query_as::<_, SchedRow>(
            r#"SELECT * FROM schedules WHERE user_id = ? ORDER BY created_at DESC"#,
        )
        .bind(user_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(persist)?;
        rows.into_iter().map(SchedRow::into_schedule).collect()
    }

    async fn delete(&self, id: ScheduleId) -> DomainResult<()> {
        sqlx::query(r#"DELETE FROM schedules WHERE id = ?"#)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(persist)?;
        Ok(())
    }

    async fn count_active(&self, user_id: UserId) -> DomainResult<u32> {
        let count: i64 = sqlx::query_scalar(
            r#"SELECT COUNT(*) FROM schedules WHERE user_id = ? AND active = 1"#,
        )
        .bind(user_id.to_string())
        .fetch_one(&self.pool)
        .await
        .map_err(persist)?;
        Ok(count as u32)
    }

    async fn due(&self, now: DateTime<Utc>, limit: i64) -> DomainResult<Vec<Schedule>> {
        let rows = sqlx::query_as::<_, SchedRow>(
            r#"SELECT * FROM schedules
               WHERE active = 1 AND next_run_at <= ?
               ORDER BY next_run_at ASC LIMIT ?"#,
        )
        .bind(now.to_rfc3339())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(persist)?;
        rows.into_iter().map(SchedRow::into_schedule).collect()
    }
}

#[derive(sqlx::FromRow)]
struct SchedRow {
    id: String,
    user_id: String,
    label: Option<String>,
    start_url: String,
    user_prompt: String,
    recurrence_json: String,
    active: i64,
    send_chat: i64,
    send_email: i64,
    send_trigger_msg: i64,
    next_run_at: String,
    last_run_at: Option<String>,
    last_crawl_id: Option<String>,
    created_at: String,
    updated_at: String,
}

impl SchedRow {
    fn into_schedule(self) -> DomainResult<Schedule> {
        let recurrence: Recurrence = serde_json::from_str(&self.recurrence_json)
            .map_err(|e| DomainError::Persistence(e.to_string()))?;
        Ok(Schedule {
            id: ScheduleId::parse(&self.id).map_err(|e| DomainError::Persistence(e.to_string()))?,
            user_id: UserId::parse(&self.user_id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            label: self.label,
            start_url: self.start_url,
            user_prompt: self.user_prompt,
            recurrence,
            active: self.active != 0,
            delivery: DeliveryFlags {
                send_chat: self.send_chat != 0,
                send_email: self.send_email != 0,
                send_trigger_message: self.send_trigger_msg != 0,
            },
            next_run_at: parse_dt(&self.next_run_at)?,
            last_run_at: self.last_run_at.as_deref().map(parse_dt).transpose()?,
            last_crawl_id: self
                .last_crawl_id
                .as_deref()
                .map(CrawlJobId::parse)
                .transpose()
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            created_at: parse_dt(&self.created_at)?,
            updated_at: parse_dt(&self.updated_at)?,
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
