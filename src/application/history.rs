use std::sync::Arc;

use crate::domain::{DomainError, DomainResult, HistoryEntry, HistoryEntryId, UserId};
use crate::ports::HistoryRepository;

pub struct HistoryService {
    pub history: Arc<dyn HistoryRepository>,
}

impl HistoryService {
    pub async fn list(
        &self,
        user_id: UserId,
        limit: i64,
        offset: i64,
    ) -> DomainResult<Vec<HistoryEntry>> {
        self.history.list_by_user(user_id, limit, offset).await
    }

    pub async fn get(&self, user_id: UserId, id: HistoryEntryId) -> DomainResult<HistoryEntry> {
        let entry = self
            .history
            .get(id)
            .await?
            .ok_or_else(|| DomainError::NotFound("history".into()))?;
        if entry.user_id != user_id {
            return Err(DomainError::NotFound("history".into()));
        }
        Ok(entry)
    }

    pub async fn list_by_schedule(
        &self,
        user_id: UserId,
        schedule_id: crate::domain::ScheduleId,
        limit: i64,
    ) -> DomainResult<Vec<HistoryEntry>> {
        self.history
            .list_by_schedule(user_id, schedule_id, limit)
            .await
    }

    pub async fn recent_successes(
        &self,
        user_id: UserId,
        limit: i64,
    ) -> DomainResult<Vec<HistoryEntry>> {
        let all = self.history.list_by_user(user_id, limit * 3, 0).await?;
        Ok(all
            .into_iter()
            .filter(|e| e.status == crate::domain::CrawlStatus::Succeeded)
            .take(limit as usize)
            .collect())
    }
}
