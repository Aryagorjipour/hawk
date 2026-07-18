use teloxide::prelude::*;
use teloxide::types::SuccessfulPayment;

use crate::adapters::telegram::state::AppState;
use crate::domain::TelegramUserId;

pub async fn handle_pre_checkout(
    bot: Bot,
    q: PreCheckoutQuery,
    _state: AppState,
) -> anyhow::Result<()> {
    // Always OK for known pack payloads; Telegram requires answer within 10s
    bot.answer_pre_checkout_query(q.id, true).await?;
    Ok(())
}

pub async fn handle_successful_payment(
    bot: Bot,
    msg: Message,
    state: AppState,
) -> anyhow::Result<()> {
    let Some(from) = msg.from.as_ref() else {
        return Ok(());
    };
    let Some(payment) = msg.successful_payment() else {
        return Ok(());
    };

    let tg_id = TelegramUserId::new(from.id.0 as i64);
    let name = from
        .username
        .clone()
        .unwrap_or_else(|| from.first_name.clone());
    let ensured = state
        .onboard
        .ensure_user(tg_id, &name, from.language_code.as_deref())
        .await
        .map_err(anyhow::Error::msg)?;
    let user = ensured.user;
    let locale = user.locale;

    apply_payment(&bot, &msg, &state, user.id, payment, locale).await
}

async fn apply_payment(
    bot: &Bot,
    msg: &Message,
    state: &AppState,
    user_id: crate::domain::UserId,
    payment: &SuccessfulPayment,
    locale: crate::domain::Locale,
) -> anyhow::Result<()> {
    let pack_id = payment.invoice_payload.as_str();
    let charge_id = payment.telegram_payment_charge_id.as_str();

    match state
        .purchases
        .apply_stars_payment(user_id, charge_id, pack_id)
        .await
    {
        Ok(p) => {
            bot.send_message(
                msg.chat.id,
                format!(
                    "{}\n+{} crawls, +{} schedule slots.",
                    state.i18n.t0(locale, "about-tip"),
                    p.credits_granted,
                    p.slots_granted
                ),
            )
            .await?;
        }
        Err(e) => {
            bot.send_message(msg.chat.id, format!("Payment bookkeeping failed: {e}"))
                .await?;
        }
    }
    Ok(())
}
