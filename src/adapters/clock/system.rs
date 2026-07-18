use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::ports::Clock;

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

#[async_trait]
impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}
