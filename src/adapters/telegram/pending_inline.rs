use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Utc};
use parking_lot::Mutex;
use uuid::Uuid;

use crate::domain::UserId;

const TTL: Duration = Duration::from_secs(600);
const MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone)]
pub struct PendingInlineCrawl {
    pub user_id: UserId,
    pub telegram_user_id: i64,
    pub url: String,
    pub prompt: String,
    pub created_at: Instant,
}

/// Telegram inline `result_id` must be short and safe charset (no `:` `/` etc).
#[derive(Default)]
pub struct PendingInlineStore {
    inner: Mutex<HashMap<String, PendingInlineCrawl>>,
}

impl PendingInlineStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a Telegram-safe result id (`c` + 32 hex).
    pub fn insert(
        &self,
        user_id: UserId,
        telegram_user_id: i64,
        url: String,
        prompt: String,
    ) -> String {
        let id = format!("c{}", Uuid::new_v4().as_simple());
        let mut map = self.inner.lock();
        Self::evict_locked(&mut map);
        if map.len() >= MAX_ENTRIES {
            // Drop oldest-ish by clearing expired first; if still full, clear half
            if map.len() >= MAX_ENTRIES {
                map.clear();
            }
        }
        map.insert(
            id.clone(),
            PendingInlineCrawl {
                user_id,
                telegram_user_id,
                url,
                prompt,
                created_at: Instant::now(),
            },
        );
        id
    }

    pub fn take(&self, id: &str) -> Option<PendingInlineCrawl> {
        let mut map = self.inner.lock();
        Self::evict_locked(&mut map);
        map.remove(id)
    }

    fn evict_locked(map: &mut HashMap<String, PendingInlineCrawl>) {
        let now = Instant::now();
        map.retain(|_, v| now.duration_since(v.created_at) < TTL);
    }
}

/// Format usage summary for chat (i18n-independent numbers; labels via caller).
pub fn usage_numbers(
    free_remaining: u32,
    free_max: u32,
    bonus_credits: u64,
    active_schedules: u32,
    max_schedules: u32,
    bonus_slots: u32,
    now: DateTime<Utc>,
) -> UsageSnapshot {
    UsageSnapshot {
        free_remaining,
        free_max,
        free_used: free_max.saturating_sub(free_remaining),
        bonus_credits,
        active_schedules,
        max_schedules,
        bonus_slots,
        total_budget: u64::from(free_remaining) + bonus_credits,
        as_of: now,
    }
}

#[derive(Debug, Clone)]
pub struct UsageSnapshot {
    pub free_remaining: u32,
    pub free_max: u32,
    pub free_used: u32,
    pub bonus_credits: u64,
    pub active_schedules: u32,
    pub max_schedules: u32,
    pub bonus_slots: u32,
    pub total_budget: u64,
    pub as_of: DateTime<Utc>,
}
