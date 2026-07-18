use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::{CrawlJobId, PaymentId, ScheduleId, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DomainEvent {
    UserOnboarded {
        user_id: UserId,
        at: DateTime<Utc>,
    },
    CrawlCompleted {
        user_id: UserId,
        crawl_id: CrawlJobId,
        at: DateTime<Utc>,
    },
    CrawlFailed {
        user_id: UserId,
        crawl_id: CrawlJobId,
        kind: String,
        at: DateTime<Utc>,
    },
    ScheduleFired {
        user_id: UserId,
        schedule_id: ScheduleId,
        crawl_id: CrawlJobId,
        at: DateTime<Utc>,
    },
    CreditsPurchased {
        user_id: UserId,
        payment_id: PaymentId,
        pack_id: String,
        at: DateTime<Utc>,
    },
    UserDataDeleted {
        user_id: UserId,
        telegram_user_id: i64,
        at: DateTime<Utc>,
    },
}

impl DomainEvent {
    pub fn name(&self) -> &'static str {
        match self {
            Self::UserOnboarded { .. } => "user_onboarded",
            Self::CrawlCompleted { .. } => "crawl_completed",
            Self::CrawlFailed { .. } => "crawl_failed",
            Self::ScheduleFired { .. } => "schedule_fired",
            Self::CreditsPurchased { .. } => "credits_purchased",
            Self::UserDataDeleted { .. } => "user_data_deleted",
        }
    }
}
