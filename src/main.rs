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

use crate::{
    database::{plugin::DatabasePluginInit, sqlite_plugin::SQLitePlugin},
    utils::responses::Responses,
};

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
            commands: vec![
                play(),
                play_single(),
                queue(),
                now_playing(),
                timestamp(),
                history(),
                play_history(),
                seek(),
                log(),
                log_file(),
                skip(),
                join(),
                leave(),
                mute(),
                unmute(),
                deafen(),
                undeafen(),
                help(),
                version(),
                register(),
            ],
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

/// Registers or unregisters application commands in this guild or globally
#[poise::command(prefix_command, hide_in_help)]
async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

// Media
#[poise::command(slash_command, prefix_command, aliases("p"), category = "media")]
async fn play(ctx: Context<'_>, #[description = "Query or url"] song: String) -> Result<(), Error> {
    media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song, true).await
}

#[poise::command(
    slash_command,
    prefix_command,
    aliases("ps", "play-single"),
    category = "media"
)]
async fn play_single(
    ctx: Context<'_>,
    #[description = "Query or url"] song: String,
) -> Result<(), Error> {
    media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song, false).await
}

#[poise::command(slash_command, prefix_command, category = "media")]
async fn skip(ctx: Context<'_>) -> Result<(), Error> {
    media::commands::skip(&GLOBAL_MEDIA_PLAYER, ctx).await
}

#[poise::command(slash_command, prefix_command, category = "media")]
async fn seek(ctx: Context<'_>, #[description = "Timestampe"] to: String) -> Result<(), Error> {
    media::commands::seek(&GLOBAL_MEDIA_PLAYER, ctx, &to).await
}

#[poise::command(slash_command, prefix_command, category = "media")]
async fn queue(ctx: Context<'_>, #[description = "Page #"] page: Option<i64>) -> Result<(), Error> {
    let page = page.unwrap_or_default();

    if page < 0 {
        ctx.error("Page cannot be less than 0").await;
        return Ok(());
    }

    media::commands::queue(&GLOBAL_MEDIA_PLAYER, ctx, page as usize).await
}

#[poise::command(
    slash_command,
    prefix_command,
    aliases("np", "now-playing"),
    category = "media"
)]
async fn now_playing(ctx: Context<'_>) -> Result<(), Error> {
    media::commands::now_playing(&GLOBAL_MEDIA_PLAYER, ctx).await
}

#[poise::command(slash_command, prefix_command, category = "media")]
async fn timestamp(ctx: Context<'_>) -> Result<(), Error> {
    media::commands::timestamp(&GLOBAL_MEDIA_PLAYER, ctx).await
}

// Playlists

#[poise::command(slash_command, prefix_command, category = "playlists")]
async fn history(
    ctx: Context<'_>,
    #[description = "Page #"] page: Option<i64>,
) -> Result<(), Error> {
    let page = page.unwrap_or_default();

    if page < 0 {
        ctx.error("Page cannot be less than 0").await;
        return Ok(());
    }

    database::commands::history(ctx, page as usize).await
}

#[poise::command(
    slash_command,
    prefix_command,
    aliases("play-history"),
    category = "playlists"
)]
async fn play_history(
    ctx: Context<'_>,
    #[description = "Index #"] index: i64,
) -> Result<(), Error> {
    if index < 0 {
        ctx.error("Index cannot be less than 0").await;
        return Ok(());
    }

    let song = database::commands::get_history(ctx, index as usize).await;
    if let Some(song) = song {
        return media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song, false).await;
    }

    Ok(())
}

// Controls

#[poise::command(slash_command, prefix_command, category = "controls")]
async fn join(ctx: Context<'_>) -> Result<(), Error> {
    controls::commands::join(&GLOBAL_MEDIA_PLAYER, ctx).await
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

// Logging
#[poise::command(slash_command, prefix_command, category = "debug")]
async fn log(
    ctx: Context<'_>,
    #[description = "Arguments"] args: Option<String>,
) -> Result<(), Error> {
    logging::commands::log(ctx, &args.unwrap_or_default()).await
}

#[poise::command(slash_command, prefix_command, aliases("log-file"), category = "debug")]
async fn log_file(ctx: Context<'_>) -> Result<(), Error> {
    logging::commands::log_file(ctx).await
}

// Help
#[poise::command(slash_command, prefix_command, aliases("v"), category = "debug")]
async fn version(ctx: Context<'_>) -> Result<(), Error> {
    ctx.info(format!("Version: {}", env!("VERGEN_GIT_SEMVER")))
        .await;

    Ok(())
}

#[poise::command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration {
            // show_context_menu_commands: true,
            ..Default::default()
        },
    )
    .await?;
    Ok(())
}
