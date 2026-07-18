pub mod callbacks;
pub mod commands;
pub mod inline;
pub mod messages;
pub mod payments;

use teloxide::dispatching::{UpdateFilterExt, UpdateHandler};
use teloxide::prelude::*;
use teloxide::types::Update;

pub fn schema() -> UpdateHandler<anyhow::Error> {
    dptree::entry()
        .branch(
            Update::filter_message()
                .branch(
                    dptree::entry()
                        .filter_command::<commands::Command>()
                        .endpoint(commands::handle),
                )
                .branch(dptree::endpoint(messages::handle_text)),
        )
        .branch(Update::filter_callback_query().endpoint(callbacks::handle))
        .branch(Update::filter_inline_query().endpoint(inline::handle_inline))
        .branch(Update::filter_chosen_inline_result().endpoint(inline::handle_chosen))
        .branch(
            Update::filter_message()
                .filter(|m: Message| m.successful_payment().is_some())
                .endpoint(payments::handle_successful_payment),
        )
        .branch(Update::filter_pre_checkout_query().endpoint(payments::handle_pre_checkout))
}
