use crate::domain::{CrawlJobId, HistoryEntryId, PaymentId, ScheduleId, UserId};

pub trait IdGenerator: Send + Sync {
    fn user_id(&self) -> UserId;
    fn crawl_id(&self) -> CrawlJobId;
    fn schedule_id(&self) -> ScheduleId;
    fn history_id(&self) -> HistoryEntryId;
    fn payment_id(&self) -> PaymentId;
}
