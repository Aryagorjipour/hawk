use chrono::{DateTime, Utc};

use super::credits::CreditPack;
use super::ids::{PaymentId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StarsPayment {
    pub id: PaymentId,
    pub user_id: UserId,
    pub telegram_payment_charge_id: String,
    pub pack_id: String,
    pub stars_amount: u32,
    pub credits_granted: u32,
    pub slots_granted: u32,
    pub created_at: DateTime<Utc>,
}

impl StarsPayment {
    pub fn from_pack(
        user_id: UserId,
        charge_id: impl Into<String>,
        pack: &CreditPack,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: PaymentId::new(),
            user_id,
            telegram_payment_charge_id: charge_id.into(),
            pack_id: pack.id.to_string(),
            stars_amount: pack.stars,
            credits_granted: pack.credits,
            slots_granted: pack.schedule_slots,
            created_at: now,
        }
    }
}
