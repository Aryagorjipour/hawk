use std::sync::Arc;

use crate::domain::{
    compute_next_run, DeliveryFlags, DomainError, DomainResult, Recurrence, Schedule, ScheduleId,
    UserId,
};
use crate::ports::{Clock, ScheduleRepository, UserRepository};

pub struct ManageScheduleService {
    pub users: Arc<dyn UserRepository>,
    pub schedules: Arc<dyn ScheduleRepository>,
    pub clock: Arc<dyn Clock>,
}

impl ManageScheduleService {
    pub async fn list(&self, user_id: UserId) -> DomainResult<(Vec<Schedule>, u32, u32)> {
        let user = self
            .users
            .get_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        let list = self.schedules.list_by_user(user_id).await?;
        let active = self.schedules.count_active(user_id).await?;
        let max = user.credits.max_schedule_slots();
        Ok((list, active, max))
    }

    pub async fn create(
        &self,
        user_id: UserId,
        url: String,
        prompt: String,
        recurrence: Recurrence,
        delivery: DeliveryFlags,
    ) -> DomainResult<Schedule> {
        let user = self
            .users
            .get_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        user.ensure_ready_to_crawl()?;
        let active = self.schedules.count_active(user_id).await?;
        if !user.credits.can_activate_schedule(active) {
            return Err(DomainError::ScheduleSlotLimit);
        }
        let now = self.clock.now();
        let schedule = Schedule::new(
            user_id,
            url,
            prompt,
            recurrence,
            delivery,
            &user.timezone,
            now,
            user.email.is_some(),
        )?;
        self.schedules.insert(&schedule).await?;
        Ok(schedule)
    }

    pub async fn set_active(
        &self,
        user_id: UserId,
        schedule_id: ScheduleId,
        active: bool,
    ) -> DomainResult<Schedule> {
        let user = self
            .users
            .get_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        let mut schedule = self
            .schedules
            .get(schedule_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("schedule".into()))?;
        if schedule.user_id != user_id {
            return Err(DomainError::NotFound("schedule".into()));
        }
        if active && !schedule.active {
            let count = self.schedules.count_active(user_id).await?;
            if !user.credits.can_activate_schedule(count) {
                return Err(DomainError::ScheduleSlotLimit);
            }
        }
        let now = self.clock.now();
        schedule.set_active(active, now);
        if active {
            schedule.next_run_at =
                compute_next_run(&schedule.recurrence, &user.timezone, now, None)?;
        }
        self.schedules.update(&schedule).await?;
        Ok(schedule)
    }

    pub async fn delete(&self, user_id: UserId, schedule_id: ScheduleId) -> DomainResult<()> {
        let schedule = self
            .schedules
            .get(schedule_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("schedule".into()))?;
        if schedule.user_id != user_id {
            return Err(DomainError::NotFound("schedule".into()));
        }
        self.schedules.delete(schedule_id).await
    }

    pub async fn get(&self, user_id: UserId, schedule_id: ScheduleId) -> DomainResult<Schedule> {
        let schedule = self
            .schedules
            .get(schedule_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("schedule".into()))?;
        if schedule.user_id != user_id {
            return Err(DomainError::NotFound("schedule".into()));
        }
        Ok(schedule)
    }
}
