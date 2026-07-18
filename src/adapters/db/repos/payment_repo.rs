use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

use crate::domain::{DomainError, DomainResult, PaymentId, StarsPayment, UserId};
use crate::ports::PaymentRepository;

pub struct SqlitePaymentRepository {
    pool: SqlitePool,
}

impl SqlitePaymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PaymentRepository for SqlitePaymentRepository {
    async fn insert(&self, payment: &StarsPayment) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO stars_payments (
                id, user_id, telegram_payment_charge_id, pack_id,
                stars_amount, credits_granted, slots_granted, created_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(payment.id.to_string())
        .bind(payment.user_id.to_string())
        .bind(&payment.telegram_payment_charge_id)
        .bind(&payment.pack_id)
        .bind(payment.stars_amount as i64)
        .bind(payment.credits_granted as i64)
        .bind(payment.slots_granted as i64)
        .bind(payment.created_at.to_rfc3339())
        .execute(&self.pool)
        .await
        .map_err(persist)?;
        Ok(())
    }

    async fn get_by_charge_id(&self, charge_id: &str) -> DomainResult<Option<StarsPayment>> {
        let row = sqlx::query_as::<_, PayRow>(
            r#"SELECT * FROM stars_payments WHERE telegram_payment_charge_id = ?"#,
        )
        .bind(charge_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(persist)?;
        row.map(PayRow::into_payment).transpose()
    }
}

#[derive(sqlx::FromRow)]
struct PayRow {
    id: String,
    user_id: String,
    telegram_payment_charge_id: String,
    pack_id: String,
    stars_amount: i64,
    credits_granted: i64,
    slots_granted: i64,
    created_at: String,
}

impl PayRow {
    fn into_payment(self) -> DomainResult<StarsPayment> {
        Ok(StarsPayment {
            id: PaymentId::parse(&self.id).map_err(|e| DomainError::Persistence(e.to_string()))?,
            user_id: UserId::parse(&self.user_id)
                .map_err(|e| DomainError::Persistence(e.to_string()))?,
            telegram_payment_charge_id: self.telegram_payment_charge_id,
            pack_id: self.pack_id,
            stars_amount: self.stars_amount as u32,
            credits_granted: self.credits_granted as u32,
            slots_granted: self.slots_granted as u32,
            created_at: parse_dt(&self.created_at)?,
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
