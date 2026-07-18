use std::sync::Arc;

use crate::domain::{DomainError, DomainEvent, DomainResult, Locale, User, UserId};
use crate::infrastructure::EventBus;
use crate::ports::{Clock, ConversationRepository, UserRepository};

pub struct SettingsService {
    pub users: Arc<dyn UserRepository>,
    pub conversations: Arc<dyn ConversationRepository>,
    pub clock: Arc<dyn Clock>,
    pub events: EventBus,
}

impl SettingsService {
    pub async fn set_name(&self, user: &mut User, name: String) -> DomainResult<()> {
        user.set_display_name(name, self.clock.now())?;
        self.users.update(user).await
    }

    pub async fn set_email(&self, user: &mut User, email: Option<String>) -> DomainResult<()> {
        user.set_email(email, self.clock.now())?;
        self.users.update(user).await
    }

    pub async fn set_timezone(&self, user: &mut User, tz: String) -> DomainResult<()> {
        user.set_timezone(tz, self.clock.now())?;
        self.users.update(user).await
    }

    pub async fn set_locale(&self, user: &mut User, locale: Locale) -> DomainResult<()> {
        user.set_locale(locale, self.clock.now());
        self.users.update(user).await
    }

    pub async fn hard_delete(&self, user_id: UserId) -> DomainResult<()> {
        let user = self
            .users
            .get_by_id(user_id)
            .await?
            .ok_or(DomainError::UserNotFound)?;
        let tg = user.telegram_user_id;
        self.conversations.delete(tg).await?;
        self.users.delete_by_id(user_id).await?;
        self.events.publish(DomainEvent::UserDataDeleted {
            user_id,
            telegram_user_id: tg.get(),
            at: self.clock.now(),
        });
        Ok(())
    }
}
