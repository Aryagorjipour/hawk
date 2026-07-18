use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{DomainError, DomainResult, TelegramUserId};
use crate::ports::{ConversationRecord, ConversationRepository};

pub struct SqliteConversationRepository {
    pool: SqlitePool,
}

impl SqliteConversationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConversationRepository for SqliteConversationRepository {
    async fn get(&self, telegram_id: TelegramUserId) -> DomainResult<Option<ConversationRecord>> {
        let row = sqlx::query_as::<_, ConvRow>(
            r#"SELECT telegram_user_id, state_kind, state_payload, updated_at
               FROM conversation_states WHERE telegram_user_id = ?"#,
        )
        .bind(telegram_id.get())
        .fetch_optional(&self.pool)
        .await
        .map_err(persist)?;
        Ok(row.map(|r| ConversationRecord {
            telegram_user_id: TelegramUserId::new(r.telegram_user_id),
            state_kind: r.state_kind,
            state_payload: r.state_payload,
            updated_at: parse_dt(&r.updated_at).unwrap_or_else(|_| Utc::now()),
        }))
    }

    async fn upsert(&self, record: &ConversationRecord) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO conversation_states (telegram_user_id, state_kind, state_payload, updated_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(telegram_user_id) DO UPDATE SET
                state_kind = excluded.state_kind,
                state_payload = excluded.state_payload,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(record.telegram_user_id.get())
        .bind(&record.state_kind)
        .bind(&record.state_payload)
        .bind(record.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn delete(&self, telegram_id: TelegramUserId) -> DomainResult<()> {
        sqlx::query(r#"DELETE FROM conversation_states WHERE telegram_user_id = ?"#)
            .bind(telegram_id.get())
            .execute(&self.pool)
            .await
            .map_err(persist)?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct ConvRow {
    telegram_user_id: i64,
    state_kind: String,
    state_payload: String,
    updated_at: String,
}

fn parse_dt(s: &str) -> DomainResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| DomainError::Persistence(e.to_string()))
}

fn persist(e: sqlx::Error) -> DomainError {
    DomainError::Persistence(e.to_string())
}
