use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tracing::info;

use super::handlers;
use super::state::AppState;

pub async fn run_bot(state: AppState) -> anyhow::Result<()> {
    let bot = state.bot.clone();
    bot.set_my_commands(handlers::commands::Command::bot_commands())
        .await?;

    info!("telegram long polling started");

    Dispatcher::builder(bot, handlers::schema())
        .dependencies(dptree::deps![state])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;

    Ok(())
}
