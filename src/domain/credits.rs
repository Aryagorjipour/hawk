use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};

pub const FREE_CRAWLS_PER_DAY: u32 = 10;
pub const FREE_SCHEDULE_SLOTS: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CreditBalance {
    pub bonus_crawl_credits: u64,
    pub bonus_schedule_slots: u32,
    pub free_crawls_used_today: u32,
    pub free_crawls_day: Option<NaiveDate>,
}

impl CreditBalance {
    pub fn max_schedule_slots(&self) -> u32 {
        FREE_SCHEDULE_SLOTS.saturating_add(self.bonus_schedule_slots)
    }

    pub fn roll_day_if_needed(&mut self, now: DateTime<Utc>) {
        let today = now.date_naive();
        if self.free_crawls_day != Some(today) {
            self.free_crawls_day = Some(today);
            self.free_crawls_used_today = 0;
        }
    }

    pub fn free_remaining(&self, now: DateTime<Utc>) -> u32 {
        let mut clone = self.clone();
        clone.roll_day_if_needed(now);
        FREE_CRAWLS_PER_DAY.saturating_sub(clone.free_crawls_used_today)
    }

    pub fn total_crawl_budget_hint(&self, now: DateTime<Utc>) -> u64 {
        u64::from(self.free_remaining(now)) + self.bonus_crawl_credits
    }

    /// Consumes one crawl credit: free daily first, then bonus.
    pub fn try_consume_crawl(&mut self, now: DateTime<Utc>) -> DomainResult<CreditSource> {
        self.roll_day_if_needed(now);
        if self.free_crawls_used_today < FREE_CRAWLS_PER_DAY {
            self.free_crawls_used_today += 1;
            return Ok(CreditSource::FreeDaily);
        }
        if self.bonus_crawl_credits > 0 {
            self.bonus_crawl_credits -= 1;
            return Ok(CreditSource::Bonus);
        }
        Err(DomainError::QuotaExceeded(format!(
            "daily free limit ({FREE_CRAWLS_PER_DAY}) and bonus credits exhausted"
        )))
    }

    pub fn can_activate_schedule(&self, active_count: u32) -> bool {
        active_count < self.max_schedule_slots()
    }

    pub fn apply_pack(&mut self, pack: &CreditPack) {
        self.bonus_crawl_credits = self
            .bonus_crawl_credits
            .saturating_add(u64::from(pack.credits));
        self.bonus_schedule_slots = self
            .bonus_schedule_slots
            .saturating_add(pack.schedule_slots);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreditSource {
    FreeDaily,
    Bonus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreditPack {
    pub id: &'static str,
    pub stars: u32,
    pub credits: u32,
    pub schedule_slots: u32,
}

pub const PACK_25: CreditPack = CreditPack {
    id: "pack_25",
    stars: 25,
    credits: 25,
    schedule_slots: 1,
};

pub const PACK_100: CreditPack = CreditPack {
    id: "pack_100",
    stars: 100,
    credits: 120,
    schedule_slots: 5,
};

pub const PACK_250: CreditPack = CreditPack {
    id: "pack_250",
    stars: 250,
    credits: 350,
    schedule_slots: 12,
};

pub const ALL_PACKS: &[CreditPack] = &[PACK_25, PACK_100, PACK_250];

pub fn pack_by_id(id: &str) -> DomainResult<&'static CreditPack> {
    ALL_PACKS
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| DomainError::NotFound(format!("credit pack {id}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn noon() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap()
    }

    #[test]
    fn consumes_free_before_bonus() {
        let mut bal = CreditBalance {
            bonus_crawl_credits: 5,
            ..Default::default()
        };
        for _ in 0..FREE_CRAWLS_PER_DAY {
            assert_eq!(
                bal.try_consume_crawl(noon()).unwrap(),
                CreditSource::FreeDaily
            );
        }
        assert_eq!(bal.try_consume_crawl(noon()).unwrap(), CreditSource::Bonus);
        assert_eq!(bal.bonus_crawl_credits, 4);
    }

    #[test]
    fn rejects_when_exhausted() {
        let mut bal = CreditBalance {
            free_crawls_used_today: FREE_CRAWLS_PER_DAY,
            free_crawls_day: Some(noon().date_naive()),
            bonus_crawl_credits: 0,
            bonus_schedule_slots: 0,
        };
        assert!(bal.try_consume_crawl(noon()).is_err());
    }

    #[test]
    fn rolls_day() {
        let mut bal = CreditBalance {
            free_crawls_used_today: FREE_CRAWLS_PER_DAY,
            free_crawls_day: Some(NaiveDate::from_ymd_opt(2026, 7, 17).unwrap()),
            bonus_crawl_credits: 0,
            bonus_schedule_slots: 0,
        };
        assert_eq!(
            bal.try_consume_crawl(noon()).unwrap(),
            CreditSource::FreeDaily
        );
        assert_eq!(bal.free_crawls_used_today, 1);
    }

    #[test]
    fn pack_grants_slots() {
        let mut bal = CreditBalance::default();
        bal.apply_pack(&PACK_100);
        assert_eq!(bal.bonus_crawl_credits, 120);
        assert_eq!(bal.max_schedule_slots(), FREE_SCHEDULE_SLOTS + 5);
    }
}
