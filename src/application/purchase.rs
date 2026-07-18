use std::sync::Arc;

use crate::domain::{pack_by_id, DomainError, DomainEvent, DomainResult, StarsPayment, UserId};
use crate::infrastructure::EventBus;
use crate::ports::{Clock, PaymentRepository, UserRepository};

pub struct PurchaseService {
    pub users: Arc<dyn UserRepository>,
    pub payments: Arc<dyn PaymentRepository>,
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
}

impl PurchaseService {
    pub async fn apply_stars_payment(
        &self,
        user_id: UserId,
        charge_id: &str,
        pack_id: &str,
    ) -> DomainResult<StarsPayment> {
        if let Some(existing) = self.payments.get_by_charge_id(charge_id).await? {
            return Ok(existing);
        }
        let pack = pack_by_id(pack_id)?;
        let mut user = self
            .users
            .get_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;

        let payment = StarsPayment::from_pack(user_id, charge_id, pack, self.clock.now());
        user.credits.apply_pack(pack);
        user.updated_at = self.clock.now();
        self.users.update(&user).await?;
        self.payments.insert(&payment).await?;
        self.events.publish(DomainEvent::CreditsPurchased {
            user_id,
            payment_id: payment.id,
            pack_id: pack.id.to_string(),
            at: self.clock.now(),
        });
        Ok(payment)
    }
}
