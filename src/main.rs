mod controls;
mod database;
mod logging;
mod media;
mod utils;

use dotenv::dotenv;
use media::global_media_player::GlobalMediaPlayer;
use poise::{command, serenity_prelude as serenity};
use songbird::SerenityInit;
use std::{env, sync::Arc};
use tracing::{error, info};
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
    dotenv().ok();
    // CLI tools
    if let Some(exit_code) = utils::cli::parse_args().await {
        std::process::exit(exit_code);
    }

    // Init Bot
    GLOBAL_MEDIA_PLAYER.init_self().await;

    // Logging
    let file_appender = tracing_appender::rolling::never("", logging::get_log_filename());
    let (file_writer, _guard) = tracing_appender::non_blocking(file_appender);

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt()
            .finish()
            .with(fmt::Layer::default().json().with_writer(file_writer)),
    )
    .expect("Unable to set global tracing subscriber");

    // Framework
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let db_plugin = Arc::new(SQLitePlugin::default());

    let framework = poise::Framework::builder()
        .client_settings(|c| c.register_songbird().register_database_plugin(db_plugin))
        .token(std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"))
        .user_data_setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                // Debug Guild
                if let Ok(guild_id) = env::var("DEBUG_GUILD_ID") {
                    let guild = ctx
                        .http
                        .get_guild(guild_id.parse().expect("DEBUG_GUILD_ID must be an integer"))
                        .await
                        .expect("Cannot find debug DEBUG_GUILD");

                    let commands_builder =
                        poise::builtins::create_application_commands(&framework.options().commands);
                    let num_commands = commands_builder.0.len();

                    guild
                        .set_application_commands(&ctx.http, |b| {
                            *b = commands_builder;
                            b
                        })
                        .await
                        .expect("Failed to set application commands");

                    info!(
                        "Set {} application commands for {}",
                        num_commands, &guild.name
                    );
                };
                // Debug Channel
                if let Ok(debug_channel) = env::var("DEBUG_CHANNEL_ID") {
                    ctx.http
                        .get_channel(
                            debug_channel
                                .parse()
                                .expect("DEBUG_CHANNEL_ID must be an integer!"),
                        )
                        .await
                        .unwrap()
                        .id()
                        .say(
                            &ctx.http,
                            format!("Bot start. Version: {}", &env!("VERGEN_GIT_SEMVER")),
                        )
                        .await
                        .expect("Failed to send startup message");
                }
                Ok(UserData {})
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                play(),
                play_single(),
                queue(),
                now_playing(),
                timestamp(),
                history(),
                play_history(),
                database::commands::playlists(),
                seek(),
                skip(),
                join(),
                leave(),
                controls::commands::mute(),
                controls::commands::unmute(),
                controls::commands::deafen(),
                controls::commands::undeafen(),
                logging::commands::log(),
                logging::commands::log_file(),
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
        .intents(intents);

    framework.run().await.unwrap();
}

// Media
#[command(
    slash_command,
    prefix_command,
    aliases("p"),
    broadcast_typing,
    category = "media"
)]
async fn play(
    ctx: Context<'_>,
    #[description = "Query or url"]
    #[rest]
    song: String,
) -> CommandResult {
    media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song, true).await
}

#[command(
    slash_command,
    prefix_command,
    rename = "play-single",
    aliases("ps"),
    broadcast_typing,
    category = "media"
)]
async fn play_single(
    ctx: Context<'_>,
    #[description = "Query or url"] song: Vec<String>,
) -> CommandResult {
    media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song.join(" "), false).await
}

#[command(slash_command, prefix_command, broadcast_typing, category = "media")]
async fn skip(ctx: Context<'_>) -> CommandResult {
    media::commands::skip(&GLOBAL_MEDIA_PLAYER, ctx).await
}

#[command(slash_command, prefix_command, category = "media")]
async fn seek(ctx: Context<'_>, #[description = "Timestampe"] to: String) -> CommandResult {
    media::commands::seek(&GLOBAL_MEDIA_PLAYER, ctx, &to).await
}

#[command(slash_command, prefix_command, category = "media")]
async fn queue(
    ctx: Context<'_>,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    let Some(page) = validate_page(ctx,page).await else {
        return Ok(());
    };

    media::commands::queue(&GLOBAL_MEDIA_PLAYER, ctx, page).await
}

#[command(
    slash_command,
    prefix_command,
    rename = "now-playing",
    aliases("np"),
    category = "media"
)]
async fn now_playing(ctx: Context<'_>) -> CommandResult {
    media::commands::now_playing(&GLOBAL_MEDIA_PLAYER, ctx).await
}

#[command(slash_command, prefix_command, category = "media")]
async fn timestamp(ctx: Context<'_>) -> CommandResult {
    media::commands::timestamp(&GLOBAL_MEDIA_PLAYER, ctx).await
}

// Playlists

#[command(slash_command, prefix_command, category = "playlists")]
async fn history(
    ctx: Context<'_>,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    let Some(page) = validate_page(ctx,page).await else {
        return Ok(());
    };

    database::commands::history(ctx, page).await
}

#[command(
    slash_command,
    prefix_command,
    rename = "play-history",
    category = "playlists"
)]
async fn play_history(
    ctx: Context<'_>,
    #[description = "Index #"]
    #[min = 1]
    index: i64,
) -> CommandResult {
    if index < 1 {
        ctx.error("Index cannot be less than 1").await;
        return Ok(());
    }

    if let Some(song) = database::commands::get_history(ctx, index as usize - 1).await {
        return media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, &song, false).await;
    }

    Ok(())
}

// Controls

#[command(slash_command, prefix_command, category = "controls")]
async fn join(ctx: Context<'_>) -> CommandResult {
    controls::join(&GLOBAL_MEDIA_PLAYER, ctx).await
}

#[command(slash_command, prefix_command, category = "controls")]
async fn leave(ctx: Context<'_>) -> CommandResult {
    controls::leave(&GLOBAL_MEDIA_PLAYER, ctx).await
}

// Misc and tools

#[command(slash_command, prefix_command, aliases("v"), category = "debug")]
async fn version(ctx: Context<'_>) -> CommandResult {
    ctx.info(format!("Version: {}", env!("VERGEN_GIT_SEMVER")))
        .await;

    Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[command(prefix_command, hide_in_help)]
async fn register(ctx: Context<'_>) -> CommandResult {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

#[command(prefix_command, track_edits, slash_command)]
async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> CommandResult {
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

// Util
async fn validate_page(ctx: Context<'_>, page: Option<i64>) -> Option<usize> {
    let page = match page {
        Some(page) => page,
        None => 1,
    };

    if page <= 0 {
        ctx.warn("Page no must be atleast 1").await;
        return None;
    }

    Some(page as usize - 1)
}
