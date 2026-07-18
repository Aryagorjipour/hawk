use chrono::{DateTime, Datelike, Duration, NaiveTime, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

use super::error::{DomainError, DomainResult};
use super::ids::{CrawlJobId, ScheduleId, UserId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Recurrence {
    Interval {
        /// Minutes between runs; minimum 15.
        every_minutes: u32,
    },
    Daily {
        /// Local time in the user's timezone (HH:MM).
        time: String,
    },
    Weekly {
        /// Weekdays as chrono numbers Mon=0 .. Sun=6 (we store names).
        days: Vec<String>,
        time: String,
    },
}

impl Recurrence {
    pub fn interval_minutes(minutes: u32) -> DomainResult<Self> {
        if minutes < 15 {
            return Err(DomainError::Validation(
                "interval must be at least 15 minutes".into(),
            ));
        }
        Ok(Self::Interval {
            every_minutes: minutes,
        })
    }

    pub fn daily(time: NaiveTime) -> Self {
        Self::Daily {
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
        }
    }

    pub fn weekly(days: Vec<Weekday>, time: NaiveTime) -> DomainResult<Self> {
        if days.is_empty() {
            return Err(DomainError::Validation(
                "weekly schedule needs at least one day".into(),
            ));
        }
        let mut names: Vec<String> = days.iter().map(|d| weekday_name(*d).to_string()).collect();
        names.sort();
        names.dedup();
        Ok(Self::Weekly {
            days: names,
            time: format!("{:02}:{:02}", time.hour(), time.minute()),
        })
    }
}

fn weekday_name(d: Weekday) -> &'static str {
    match d {
        Weekday::Mon => "mon",
        Weekday::Tue => "tue",
        Weekday::Wed => "wed",
        Weekday::Thu => "thu",
        Weekday::Fri => "fri",
        Weekday::Sat => "sat",
        Weekday::Sun => "sun",
    }
}

fn parse_weekday(s: &str) -> DomainResult<Weekday> {
    match s.to_ascii_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        other => Err(DomainError::Validation(format!("unknown weekday: {other}"))),
    }
}

pub fn parse_hhmm(s: &str) -> DomainResult<NaiveTime> {
    let parts: Vec<&str> = s.trim().split(':').collect();
    if parts.len() != 2 {
        return Err(DomainError::Validation("time must be HH:MM".into()));
    }
    let h: u32 = parts[0]
        .parse()
        .map_err(|_| DomainError::Validation("invalid hour".into()))?;
    let m: u32 = parts[1]
        .parse()
        .map_err(|_| DomainError::Validation("invalid minute".into()))?;
    NaiveTime::from_hms_opt(h, m, 0).ok_or_else(|| DomainError::Validation("invalid time".into()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryFlags {
    pub send_chat: bool,
    pub send_email: bool,
    pub send_trigger_message: bool,
}

impl DeliveryFlags {
    pub fn chat_only() -> Self {
        Self {
            send_chat: true,
            send_email: false,
            send_trigger_message: true,
        }
    }

    pub fn validate(&self, has_email: bool) -> DomainResult<()> {
        if !self.send_chat && !self.send_email {
            return Err(DomainError::Validation(
                "enable at least chat or email delivery".into(),
            ));
        }
        if self.send_email && !has_email {
            return Err(DomainError::Validation(
                "set an email in Settings before enabling email delivery".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub id: ScheduleId,
    pub user_id: UserId,
    pub label: Option<String>,
    pub start_url: String,
    pub user_prompt: String,
    pub recurrence: Recurrence,
    pub active: bool,
    pub delivery: DeliveryFlags,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub last_crawl_id: Option<CrawlJobId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Schedule {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        user_id: UserId,
        start_url: String,
        user_prompt: String,
        recurrence: Recurrence,
        delivery: DeliveryFlags,
        timezone: &str,
        now: DateTime<Utc>,
        has_email: bool,
    ) -> DomainResult<Self> {
        delivery.validate(has_email)?;
        if start_url.trim().is_empty() || user_prompt.trim().is_empty() {
            return Err(DomainError::Validation(
                "url and prompt are required".into(),
            ));
        }
        let next = compute_next_run(&recurrence, timezone, now, None)?;
        Ok(Self {
            id: ScheduleId::new(),
            user_id,
            label: None,
            start_url: start_url.trim().to_string(),
            user_prompt: user_prompt.trim().to_string(),
            recurrence,
            active: true,
            delivery,
            next_run_at: next,
            last_run_at: None,
            last_crawl_id: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn bump_after_run(
        &mut self,
        crawl_id: CrawlJobId,
        timezone: &str,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        self.last_run_at = Some(now);
        self.last_crawl_id = Some(crawl_id);
        self.next_run_at = compute_next_run(&self.recurrence, timezone, now, Some(now))?;
        self.updated_at = now;
        Ok(())
    }

    pub fn set_active(&mut self, active: bool, now: DateTime<Utc>) {
        self.active = active;
        self.updated_at = now;
    }
}

pub fn resolve_tz(name: &str) -> DomainResult<Tz> {
    if name == "UTC" {
        return Ok(chrono_tz::UTC);
    }
    name.parse::<Tz>()
        .map_err(|_| DomainError::Validation(format!("unknown timezone: {name}")))
}

/// Compute next fire time in UTC.
/// After a run, pass `Some(now)` as `after` so interval schedules advance from now.
pub fn compute_next_run(
    recurrence: &Recurrence,
    timezone: &str,
    now: DateTime<Utc>,
    after: Option<DateTime<Utc>>,
) -> DomainResult<DateTime<Utc>> {
    let tz = resolve_tz(timezone)?;
    let from = after.unwrap_or(now);

    match recurrence {
        Recurrence::Interval { every_minutes } => {
            let mins = i64::from(*every_minutes);
            Ok(from + Duration::minutes(mins))
        }
        Recurrence::Daily { time } => {
            let t = parse_hhmm(time)?;
            next_daily(from, t, tz)
        }
        Recurrence::Weekly { days, time } => {
            let t = parse_hhmm(time)?;
            let mut weekdays = Vec::new();
            for d in days {
                weekdays.push(parse_weekday(d)?);
            }
            next_weekly(from, &weekdays, t, tz)
        }
    }
}

fn next_daily(from: DateTime<Utc>, time: NaiveTime, tz: Tz) -> DomainResult<DateTime<Utc>> {
    let local = from.with_timezone(&tz);
    let mut candidate_date = local.date_naive();
    let mut candidate = candidate_date
        .and_time(time)
        .and_local_timezone(tz)
        .single()
        .ok_or_else(|| DomainError::Internal("ambiguous local time".into()))?;

    if candidate <= local {
        candidate_date = candidate_date
            .succ_opt()
            .ok_or_else(|| DomainError::Internal("date overflow".into()))?;
        candidate = candidate_date
            .and_time(time)
            .and_local_timezone(tz)
            .single()
            .ok_or_else(|| DomainError::Internal("ambiguous local time".into()))?;
    }
    Ok(candidate.with_timezone(&Utc))
}

fn next_weekly(
    from: DateTime<Utc>,
    days: &[Weekday],
    time: NaiveTime,
    tz: Tz,
) -> DomainResult<DateTime<Utc>> {
    let local = from.with_timezone(&tz);
    for offset in 0..8 {
        let date = local
            .date_naive()
            .checked_add_signed(Duration::days(offset))
            .ok_or_else(|| DomainError::Internal("date overflow".into()))?;
        if !days.contains(&date.weekday()) {
            continue;
        }
        let candidate = date
            .and_time(time)
            .and_local_timezone(tz)
            .single()
            .ok_or_else(|| DomainError::Internal("ambiguous local time".into()))?;
        if candidate > local {
            return Ok(candidate.with_timezone(&Utc));
        }
    }
    Err(DomainError::Internal(
        "could not compute next weekly run".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn interval_from_now() {
        let now = Utc.with_ymd_and_hms(2026, 7, 18, 12, 0, 0).unwrap();
        let rec = Recurrence::interval_minutes(60).unwrap();
        let next = compute_next_run(&rec, "UTC", now, Some(now)).unwrap();
        assert_eq!(next, now + Duration::hours(1));
    }

    #[test]
    fn daily_advances_to_tomorrow() {
        let now = Utc.with_ymd_and_hms(2026, 7, 18, 15, 0, 0).unwrap();
        let rec = Recurrence::daily(NaiveTime::from_hms_opt(9, 0, 0).unwrap());
        let next = compute_next_run(&rec, "UTC", now, None).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 7, 19, 9, 0, 0).unwrap());
    }
}
