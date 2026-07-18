use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    CrawlJob, CrawlJobId, CrawlStatus, HistoryEntry, Schedule, ScheduleId, StarsPayment,
    TelegramUserId, User, UserId,
};

#[async_trait]
pub trait UserRepository: Send + Sync {
    async fn get_by_id(&self, id: UserId) -> crate::domain::DomainResult<Option<User>>;
    async fn get_by_telegram_id(
        &self,
        telegram_id: TelegramUserId,
    ) -> crate::domain::DomainResult<Option<User>>;
    async fn insert(&self, user: &User) -> crate::domain::DomainResult<()>;
    async fn update(&self, user: &User) -> crate::domain::DomainResult<()>;
    async fn delete_by_id(&self, id: UserId) -> crate::domain::DomainResult<()>;
}

#[async_trait]
pub trait ConversationRepository: Send + Sync {
    async fn get(
        &self,
        telegram_id: TelegramUserId,
    ) -> crate::domain::DomainResult<Option<ConversationRecord>>;
    async fn upsert(&self, record: &ConversationRecord) -> crate::domain::DomainResult<()>;
    async fn delete(&self, telegram_id: TelegramUserId) -> crate::domain::DomainResult<()>;
}

#[derive(Debug, Clone)]
pub struct ConversationRecord {
    pub telegram_user_id: TelegramUserId,
    pub state_kind: String,
    pub state_payload: String,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait CrawlRepository: Send + Sync {
    async fn insert(&self, job: &CrawlJob) -> crate::domain::DomainResult<()>;
    async fn update(&self, job: &CrawlJob) -> crate::domain::DomainResult<()>;
    async fn get(&self, id: CrawlJobId) -> crate::domain::DomainResult<Option<CrawlJob>>;
    async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> crate::domain::DomainResult<Vec<CrawlJob>>;
    async fn has_active_for_user(&self, user_id: UserId) -> crate::domain::DomainResult<bool>;
    async fn claim_next_queued(&self) -> crate::domain::DomainResult<Option<CrawlJob>>;
    async fn count_by_user_status(
        &self,
        user_id: UserId,
        status: CrawlStatus,
    ) -> crate::domain::DomainResult<i64>;
}

#[async_trait]
pub trait ScheduleRepository: Send + Sync {
    async fn insert(&self, schedule: &Schedule) -> crate::domain::DomainResult<()>;
    async fn update(&self, schedule: &Schedule) -> crate::domain::DomainResult<()>;
    async fn get(&self, id: ScheduleId) -> crate::domain::DomainResult<Option<Schedule>>;
    async fn list_by_user(&self, user_id: UserId) -> crate::domain::DomainResult<Vec<Schedule>>;
    async fn delete(&self, id: ScheduleId) -> crate::domain::DomainResult<()>;
    async fn count_active(&self, user_id: UserId) -> crate::domain::DomainResult<u32>;
    async fn due(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> crate::domain::DomainResult<Vec<Schedule>>;
}

#[async_trait]
pub trait HistoryRepository: Send + Sync {
    async fn insert(&self, entry: &HistoryEntry) -> crate::domain::DomainResult<()>;
    async fn list_by_user(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> crate::domain::DomainResult<Vec<HistoryEntry>>;
    async fn get(
        &self,
        id: crate::domain::HistoryEntryId,
    ) -> crate::domain::DomainResult<Option<HistoryEntry>>;
    async fn list_by_schedule(
        &self,
        user_id: UserId,
        schedule_id: ScheduleId,
        limit: i64,
    ) -> crate::domain::DomainResult<Vec<HistoryEntry>>;
    async fn trim_to_cap(&self, user_id: UserId, cap: i64) -> crate::domain::DomainResult<()>;
}

#[async_trait]
pub trait PaymentRepository: Send + Sync {
    async fn insert(&self, payment: &StarsPayment) -> crate::domain::DomainResult<()>;
    async fn get_by_charge_id(
        &self,
        charge_id: &str,
    ) -> crate::domain::DomainResult<Option<StarsPayment>>;
}

#[async_trait]
pub trait PageTraceRepository: Send + Sync {
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
    ) -> crate::domain::DomainResult<()>;
}
