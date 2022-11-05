mod controls;
mod database;
mod logging;
mod media;
mod utils;

use dotenv::dotenv;
use media::global_media_player::GlobalMediaPlayer;
use std::{collections::HashSet, env, sync::Arc};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt};
use utils::{config, message_context};

use songbird::SerenityInit;

use serenity::{
    async_trait,
    builder::CreateApplicationCommands,
    client::{Client, Context},
    framework::{
        standard::{
            help_commands,
            macros::{command, group, help},
            Args, CommandGroup, CommandResult, Delimiter, HelpOptions,
        },
        StandardFramework,
    },
    model::{
        channel::Message,
        prelude::{command::Command, interaction::Interaction, GuildId, Ready, UserId},
    },
    prelude::{EventHandler, GatewayIntents},
};

use crate::{
    database::{plugin::DatabasePluginInit, sqlite_plugin::SQLitePlugin},
    message_context::MessageContext,
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        // Register application commands
        if cfg!(debug_assertions) {
            if let Ok(guild_id) = env::var("DEBUG_GUILD_ID") {
                let _ = GuildId::set_application_commands(
                    &GuildId(
                        guild_id
                            .parse()
                            .expect("DEBUG_GUILD_ID must be an integer!"),
                    ),
                    &ctx.http,
                    register_interactions,
                )
                .await;
            }
        } else {
            let _ =
                Command::set_global_application_commands(&ctx.http, register_interactions).await;
        };

        // Debug Channel
        if let Ok(debug_channel) = env::var("DEBUG_CHANNEL_ID") {
            let _ = ctx
                .http
                .get_channel(
                    debug_channel
                        .parse::<u64>()
                        .expect("DEBUG_CHANNEL_ID must be an integer!"),
                )
                .await
                .unwrap()
                .id()
                .say(
                    &ctx.http,
                    format!(
                        "{} is connected. Version: {}",
                        ready.user.name,
                        env!("VERGEN_GIT_SEMVER")
                    ),
                )
                .await;
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        on_interaction(ctx, interaction).await
    }
}

/// Register slash commands
fn register_interactions(f: &mut CreateApplicationCommands) -> &mut CreateApplicationCommands {
    use database::commands::interaction::{
        playlist_create::COMMAND_NAME as CREATE_PLAYLIST_ID,
        playlist_list::COMMAND_NAME as LIST_PLAYLIST_ID,
    };

    f.create_application_command(|c| c.name(CREATE_PLAYLIST_ID).description("Create a playlist"))
        .create_application_command(|c| c.name(LIST_PLAYLIST_ID).description("List of playlists"))
}

/// Slash command event handler
async fn on_interaction(ctx: Context, interaction: Interaction) {
    use database::commands::interaction::{playlist_create, playlist_list};

    match interaction {
        Interaction::ApplicationCommand(command) => {
            match command.data.name.as_str() {
                playlist_create::COMMAND_NAME => playlist_create::modal(ctx, command).await,
                playlist_list::COMMAND_NAME => playlist_list::response(ctx, command).await,
                &_ => (),
            };
        }
        Interaction::ModalSubmit(modal_inter) => match modal_inter.data.custom_id.as_str() {
            playlist_create::SUBMIT_ID => playlist_create::submit(ctx, modal_inter).await,
            &_ => (),
        },
        _ => (),
    }
}

#[group]
#[commands(play, play_single, skip, queue, now_playing, seek, timestamp)]
struct Media;

#[group]
#[commands(history, play_history)]
struct History;

#[group]
#[commands(log, log_file)]
struct Log;

#[group]
#[commands(version, ping, deafen, join, leave, mute, undeafen, unmute)]
struct Controls;

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

    let token = match env::var("DISCORD_TOKEN") {
        Ok(var) => var,
        Err(_) => {
            info!("[Warning] No DISCORD_TOKEN environment variable present. Have you set the correct environment variables?\n\tSee the README for a list of available environment variables.");
            return;
        }
    };

    let prefix = env::var("BOT_PREFIX").unwrap_or("~".to_owned());

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(prefix))
        .group(&MEDIA_GROUP)
        .group(&HISTORY_GROUP)
        .group(&LOG_GROUP)
        .group(&CONTROLS_GROUP)
        .help(&HELP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let db_plugin = SQLitePlugin::default();

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .register_database_plugin(Arc::new(db_plugin))
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| info!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to shutdown correctly");

    info!("Received Ctrl-C, shutting down.");
}

#[help]
async fn help(
    context: &Context,
    msg: &Message,
    args: Args,
    help_options: &'static HelpOptions,
    groups: &[&'static CommandGroup],
    owners: HashSet<UserId>,
) -> CommandResult {
    let _ = help_commands::with_embeds(context, msg, args, help_options, groups, owners).await;
    Ok(())
}

#[command]
async fn version(ctx: &Context, msg: &Message) -> CommandResult {
    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    message_ctx
        .send_info(format!("Version: {}", env!("VERGEN_GIT_SEMVER")))
        .await;

    Ok(())
}

#[command]
#[description = "Plays a song"]
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let typing_res = msg.channel_id.start_typing(&ctx.http);
    let res = media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, msg, args, true).await;

    if let Ok(typing) = typing_res {
        let _ = typing.stop();
    }

    res
}

#[command]
#[only_in(guilds)]
async fn play_single(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let typing_res = msg.channel_id.start_typing(&ctx.http);
    let res = media::commands::play_command(&GLOBAL_MEDIA_PLAYER, ctx, msg, args, false).await;

    if let Ok(typing) = typing_res {
        let _ = typing.stop();
    }

    res
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    media::commands::skip(&GLOBAL_MEDIA_PLAYER, ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    media::commands::seek(&GLOBAL_MEDIA_PLAYER, ctx, msg, args).await
}

#[command]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    media::commands::queue(&GLOBAL_MEDIA_PLAYER, ctx, msg, args).await
}

#[command]
#[aliases(np)]
#[only_in(guilds)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    media::commands::now_playing(&GLOBAL_MEDIA_PLAYER, ctx, msg).await
}

#[command]
#[aliases(timestamps)]
#[only_in(guilds)]
async fn timestamp(ctx: &Context, msg: &Message) -> CommandResult {
    media::commands::timestamp(&GLOBAL_MEDIA_PLAYER, ctx, msg).await
}

#[command]
async fn history(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    database::commands::history(ctx, msg, args).await
}

#[command]
#[aliases("play-history")]
#[only_in(guilds)]
async fn play_history(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    if let Some(url) = database::commands::play_history(ctx, msg, args).await {
        return play(ctx, msg, Args::new(&url, &[Delimiter::Single(' ')])).await;
    }

    Ok(())
}

#[command]
async fn log(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    logging::commands::log(ctx, msg, args).await
}

#[command]
async fn log_file(ctx: &Context, msg: &Message) -> CommandResult {
    logging::commands::log_file(ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::join(&GLOBAL_MEDIA_PLAYER, ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::leave(&GLOBAL_MEDIA_PLAYER, ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::mute(ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::deafen(ctx, msg).await
}

#[command]
async fn ping(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::ping(ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::undeafen(ctx, msg).await
}

#[command]
#[only_in(guilds)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    controls::commands::unmute(ctx, msg).await
}
