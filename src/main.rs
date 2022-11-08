mod controls;
mod database;
mod logging;
mod media;
mod utils;

use dotenv::dotenv;
use media::global_media_player::GlobalMediaPlayer;
use poise::serenity_prelude as serenity;
use songbird::SerenityInit;
use std::{env, sync::Arc};
use tracing_subscriber::{fmt, layer::SubscriberExt};
use utils::{config, message_context};

use crate::database::{plugin::DatabasePluginInit, sqlite_plugin::SQLitePlugin};

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type CommandResult = Result<(), Error>;
type Context<'a> = poise::Context<'a, UserData, Error>;
pub struct UserData {}

static GLOBAL_MEDIA_PLAYER: GlobalMediaPlayer = GlobalMediaPlayer::UNINITIALIZED;

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    GLOBAL_MEDIA_PLAYER.init_self().await;

    let file_appender = tracing_appender::rolling::never("", logging::get_log_filename());
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .finish()
            .with(fmt::Layer::default().json().with_writer(file_writer)),
    )
    .expect("Unable to set global tracing subscriber");

    dotenv().ok();

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let db_plugin = Arc::new(SQLitePlugin::default());

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![join(), leave(), mute(), unmute(), deafen(), undeafen()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: env::var("BOT_PREFIX").ok(),
                ..Default::default()
            },
            ..Default::default()
        })
        .user_data_setup(move |_ctx, _ready, _framework| Box::pin(async move { Ok(UserData {}) }))
        .client_settings(|c| c.register_songbird().register_database_plugin(db_plugin))
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .intents(intents);

    framework.run().await.unwrap();
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn join(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::join(&GLOBAL_MEDIA_PLAYER, &ctx).await
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn leave(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::leave(&GLOBAL_MEDIA_PLAYER, &ctx).await
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn mute(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::mute(&ctx).await
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn unmute(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::unmute(&ctx).await
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn deafen(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::deafen(&ctx).await
}

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn undeafen(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::undeafen(&ctx).await
}
