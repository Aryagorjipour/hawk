use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::SqlitePool;

use crate::domain::{
    AiConfig, CreditBalance, DomainError, DomainResult, EncryptedBlob, Locale, ModelId,
    OnboardingStatus, ProviderKind, TelegramUserId, User, UserId,
};
use crate::ports::UserRepository;

pub struct SqliteUserRepository {
    pool: SqlitePool,
}

impl SqliteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepository for SqliteUserRepository {
    async fn get_by_id(&self, id: UserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(r#"SELECT * FROM users WHERE id = ?"#)
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(persist)?;
        row.map(UserRow::into_user).transpose()
    }

    async fn get_by_telegram_id(&self, telegram_id: TelegramUserId) -> DomainResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(r#"SELECT * FROM users WHERE telegram_user_id = ?"#)
            .bind(telegram_id.get())
            .fetch_optional(&self.pool)
            .await
            .map_err(persist)?;
        row.map(UserRow::into_user).transpose()
    }

    async fn insert(&self, user: &User) -> DomainResult<()> {
        let (provider, base_url, ct, nonce, model, verified) = ai_cols(&user.ai_config);
        sqlx::query(
            r#"
            INSERT INTO users (
                id, telegram_user_id, display_name, email, timezone, locale,
                onboarding_status, provider, base_url, api_key_ciphertext, api_key_nonce,
                model_id, connection_verified_at, bonus_crawl_credits, bonus_schedule_slots,
                free_crawls_used_today, free_crawls_day, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(user.id.to_string())
        .bind(user.telegram_user_id.get())
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(&user.timezone)
        .bind(user.locale.as_str())
        .bind(user.onboarding_status.as_str())
        .bind(provider)
        .bind(base_url)
        .bind(ct)
        .bind(nonce)
        .bind(model)
        .bind(verified)
        .bind(user.credits.bonus_crawl_credits as i64)
        .bind(user.credits.bonus_schedule_slots as i64)
        .bind(user.credits.free_crawls_used_today as i64)
        .bind(user.credits.free_crawls_day.map(|d| d.to_string()))
        .bind(user.created_at.to_rfc3339())
        .bind(user.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn update(&self, user: &User) -> DomainResult<()> {
        let (provider, base_url, ct, nonce, model, verified) = ai_cols(&user.ai_config);
        let res = sqlx::query(
            r#"
            UPDATE users SET
                display_name = ?, email = ?, timezone = ?, locale = ?,
                onboarding_status = ?, provider = ?, base_url = ?,
                api_key_ciphertext = ?, api_key_nonce = ?, model_id = ?,
                connection_verified_at = ?, bonus_crawl_credits = ?,
                bonus_schedule_slots = ?, free_crawls_used_today = ?,
                free_crawls_day = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&user.display_name)
        .bind(&user.email)
        .bind(&user.timezone)
        .bind(user.locale.as_str())
        .bind(user.onboarding_status.as_str())
        .bind(provider)
        .bind(base_url)
        .bind(ct)
        .bind(nonce)
        .bind(model)
        .bind(verified)
        .bind(user.credits.bonus_crawl_credits as i64)
        .bind(user.credits.bonus_schedule_slots as i64)
        .bind(user.credits.free_crawls_used_today as i64)
        .bind(user.credits.free_crawls_day.map(|d| d.to_string()))
        .bind(user.updated_at.to_rfc3339())
        .bind(user.id.to_string())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }
        Ok(())
    }

    async fn delete_by_id(&self, id: UserId) -> DomainResult<()> {
        let res = sqlx::query(r#"DELETE FROM users WHERE id = ?"#)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(persist)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::UserNotFound);
        }
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: String,
    telegram_user_id: i64,
    display_name: String,
    email: Option<String>,
    timezone: String,
    locale: String,
    onboarding_status: String,
    provider: Option<String>,
    base_url: Option<String>,
    api_key_ciphertext: Option<Vec<u8>>,
    api_key_nonce: Option<Vec<u8>>,
    model_id: Option<String>,
    connection_verified_at: Option<String>,
    bonus_crawl_credits: i64,
    bonus_schedule_slots: i64,
    free_crawls_used_today: i64,
    free_crawls_day: Option<String>,
    created_at: String,
    updated_at: String,
}

impl UserRow {
    fn into_user(self) -> DomainResult<User> {
        let ai_config = match (
            self.provider,
            self.base_url,
            self.api_key_ciphertext,
            self.api_key_nonce,
            self.model_id,
        ) {
            (Some(p), Some(base), Some(ct), Some(nonce), Some(model)) => Some(AiConfig {
                provider: ProviderKind::parse(&p)?,
                base_url: base,
                api_key: EncryptedBlob {
                    nonce,
                    ciphertext: ct,
                },
                model_id: ModelId::new(model)?,
                connection_verified_at: self
                    .connection_verified_at
                    .as_deref()
                    .map(parse_dt)
                    .transpose()?,
            }),
            _ => None,
        };

        Ok(User {
            id: UserId::parse(&self.id).map_err(|e| DomainError::Persistence(e.to_string()))?,
            telegram_user_id: TelegramUserId::new(self.telegram_user_id),
            display_name: self.display_name,
            email: self.email,
            timezone: self.timezone,
            locale: Locale::parse(&self.locale),
            onboarding_status: OnboardingStatus::parse(&self.onboarding_status)?,
            ai_config,
            credits: CreditBalance {
                bonus_crawl_credits: self.bonus_crawl_credits as u64,
                bonus_schedule_slots: self.bonus_schedule_slots as u32,
                free_crawls_used_today: self.free_crawls_used_today as u32,
                free_crawls_day: self
                    .free_crawls_day
                    .as_deref()
                    .map(|s| {
                        NaiveDate::parse_from_str(s, "%Y-%m-%d")
                            .map_err(|e| DomainError::Persistence(e.to_string()))
                    })
                    .transpose()?,
            },
            created_at: parse_dt(&self.created_at)?,
            updated_at: parse_dt(&self.updated_at)?,
        })
    }
}

type AiCols = (
    Option<String>,
    Option<String>,
    Option<Vec<u8>>,
    Option<Vec<u8>>,
    Option<String>,
    Option<String>,
);

fn ai_cols(cfg: &Option<AiConfig>) -> AiCols {
    match cfg {
        Some(c) => (
            Some(c.provider.as_str().to_string()),
            Some(c.base_url.clone()),
            Some(c.api_key.ciphertext.clone()),
            Some(c.api_key.nonce.clone()),
            Some(c.model_id.as_str().to_string()),
            c.connection_verified_at.map(|t| t.to_rfc3339()),
        ),
        None => (None, None, None, None, None, None),
    }
}

fn parse_dt(s: &str) -> DomainResult<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| DomainError::Persistence(format!("bad datetime {s}: {e}")))
}

fn persist(e: sqlx::Error) -> DomainError {
    DomainError::Persistence(e.to_string())
}
