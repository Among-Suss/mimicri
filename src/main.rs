mod controls;
mod database;
mod logging;
mod media;
mod utils;

use dotenv::dotenv;
use media::{global_media_player::GlobalMediaPlayer, plugin::GlobalMediaPlayerPluginInit};
use poise::{command, serenity_prelude as serenity};
use songbird::SerenityInit;
use std::{env, sync::Arc};
use tracing::info;
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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    dotenv().ok();

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
    let db_plugin = Arc::new(SQLitePlugin::default());
    let media_player_plugin = Arc::new(GlobalMediaPlayer::UNINITIALIZED);
    media_player_plugin.init_self().await;

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .client_settings(|c| {
            c.register_songbird()
                .register_database_plugin(db_plugin)
                .register_media_player_plugin(media_player_plugin)
        })
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
                            format!("Bot started! Version: {}", &env!("VERGEN_GIT_SEMVER")),
                        )
                        .await
                        .expect("Failed to send startup message");
                }
                Ok(UserData {})
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                media::commands::play(),
                media::commands::play_single(),
                media::commands::play_next(),
                media::commands::seek(),
                media::commands::skip(),
                media::commands::queue(),
                media::commands::clear(),
                media::commands::now_playing(),
                media::commands::timestamp(),
                database::commands::history(),
                database::commands::playlists(),
                controls::commands::join(),
                controls::commands::leave(),
                controls::commands::mute(),
                controls::commands::unmute(),
                controls::commands::deafen(),
                controls::commands::undeafen(),
                logging::commands::log(),
                logging::commands::log_file(),
                update(),
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

// Misc and tools

#[command(prefix_command, aliases("v"), category = "debug")]
async fn version(ctx: Context<'_>) -> CommandResult {
    ctx.info(format!("Version: {}", env!("VERGEN_GIT_SEMVER")))
        .await;

    Ok(())
}

#[command(prefix_command, category = "debug")]
async fn update(ctx: Context<'_>) -> CommandResult {
    // spawn yt-dlp --update
    let cmd = std::process::Command::new("yt-dlp")
        .arg("--update")
        .output()
        .expect("failed to execute process");

    ctx.send(|m| m.content(String::from_utf8(cmd.stdout).unwrap()))
        .await
        .expect("Failed to send message");

    CommandResult::Ok(())
}

/// Registers or unregisters application commands in this guild or globally
#[command(prefix_command, hide_in_help)]
async fn register(ctx: Context<'_>) -> CommandResult {
    poise::builtins::register_application_commands_buttons(ctx).await?;

    Ok(())
}

/// Show help
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
