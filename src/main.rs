mod controls;
mod database;
mod logging;
mod media;
mod play;
mod utils;

use dotenv::dotenv;
use media::media::GlobalMediaPlayer;
use media::metadata;
use std::{cmp, env, sync::Arc};
use tracing::{info, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt};
use utils::{config, message_context, strings};

use songbird::SerenityInit;

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult, Delimiter,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::GatewayIntents,
};

use crate::{
    database::{
        plugin::{get_db_plugin, DatabasePluginInit},
        sqlite_plugin::SQLitePlugin,
    },
    message_context::MessageContext,
    metadata::parse_timestamps,
    play::queue_variant,
    strings::{
        create_progress_bar, escape_string, format_timestamp, is_timestamp, limit_string_length,
        parse_timestamp,
    },
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} is connected!", ready.user.name);

        if let Ok(debug_channel) = env::var("DEBUG_CHANNEL_ID") {
            let id = debug_channel.parse::<u64>().unwrap();

            let _ = ctx
                .http
                .get_channel(id)
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
}

#[group]
#[commands(
    play,
    play_single,
    skip,
    queue,
    now_playing,
    seek,
    timestamp,
    // history/playlist
    history,
    play_history,
    // debug
    log,
    log_file,
    // etc
    version,
    ping,
    deafen,
    join,
    leave,
    mute,
    undeafen,
    unmute,
)]
struct General;

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
        .group(&GENERAL_GROUP);

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
#[aliases(p)]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext::new(ctx, msg);

    // Join vc
    let user_vc = match guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(vc) => vc,
        None => {
            message_ctx
                .reply_error(msg, "Ur not even in a vc idio")
                .await;
            return Ok(());
        }
    };

    match guild
        .voice_states
        .get(&ctx.cache.current_user_id())
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(bot_vc) => {
            if bot_vc != user_vc {
                message_ctx.reply_error(msg, "Wrong channel dumbass").await;

                return Ok(());
            }
        }
        None => {
            match join(ctx, msg, args.clone()).await {
                Ok(_) => (),
                Err(err) => message_ctx.reply_error(msg, err).await,
            };
        }
    }

    // Get url
    let url = args.raw().collect::<Vec<&str>>().join(" ");

    if url.eq("") {
        message_ctx
            .reply_error(msg, "You didn't send anything dumbass")
            .await;

        return Ok(());
    }

    let db_plugin = get_db_plugin(ctx).await.unwrap().clone();

    match queue_variant(guild_id, &url, message_ctx.clone(), &GLOBAL_MEDIA_PLAYER).await {
        Ok(info) => {
            message_ctx
                .send_message(|m| {
                    m.content("")
                        .embed(|e| {
                            e.title(&info.title)
                                .author(|a| a.name("Queued:"))
                                // .description(format!("[{}]({})", info.title, info.url))
                                .thumbnail(&info.thumbnail)
                                .url(&info.url)
                                .color(config::colors::play())
                        })
                        .reference_message(msg);

                    m
                })
                .await;

            let _ = db_plugin.set_history(*msg.author.id.as_u64(), info);
        }
        Err(err) => message_ctx.reply_error(msg, err).await,
    }

    Ok(())
}

// FIXME not working because it's not content, it's args
#[command]
#[only_in(guilds)]
async fn play_single(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let list_index = msg.content.find("&list=");

    let mut new_msg = msg.clone();

    if let Some(ind) = list_index {
        new_msg.content = msg.content.clone()[0..ind].to_string();
    }

    play(ctx, &new_msg, args).await?;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let res = GLOBAL_MEDIA_PLAYER.skip(guild_id).await;

    let message_context = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    match res {
        Ok(_) => {
            message_context
                .reply_info(msg, "Skipped current song!")
                .await
        }
        Err(err) => message_context.reply_error(msg, err).await,
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn seek(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let message_context = MessageContext::new(ctx, msg);

    let arg_1 = args.raw().nth(0).unwrap_or_default().to_string();
    let guild_id = msg.guild_id.unwrap();

    let time = if let Ok(seconds) = arg_1.parse::<i64>() {
        seconds
    } else if is_timestamp(&arg_1) {
        parse_timestamp(&arg_1)
    } else {
        message_context
            .reply_error(msg, format!("{} isn't a valid timestamp.", arg_1))
            .await;

        return Ok(());
    };

    if time < 0 {
        message_context
            .reply_error(msg, "Cannot seek to negative time.")
            .await;

        return Ok(());
    }

    match GLOBAL_MEDIA_PLAYER.seek(guild_id, time).await {
        Ok(_) => {
            message_context
                .reply_info(msg, format!("Seeking to {}", format_timestamp(time)))
                .await;
        }
        Err(err) => {
            message_context.reply_error(msg, &err).await;
            warn!("Seek error: {}", &err);
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let page = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    let queue_page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    let res = GLOBAL_MEDIA_PLAYER
        .read_queue(guild_id, page * queue_page_size, queue_page_size)
        .await;

    match res {
        Ok((queue, len)) => {
            if len == 0 {
                message_ctx.reply_info(msg, "The queue is empty!").await;
                return Ok(());
            }

            message_ctx
                .send_message(|m| {
                    m.content("").embed(|e| {
                        e.title("Queue")
                            .description(
                                queue
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, info)| {
                                        format!(
                                            "**{}) [{}]({})** ({})",
                                            i + 1 + page * queue_page_size,
                                            escape_string(&limit_string_length(
                                                &info.title,
                                                queue_text_len,
                                            )),
                                            info.url,
                                            format_timestamp(info.duration)
                                        )
                                    })
                                    .collect::<Vec<String>>()
                                    .join("\n"),
                            )
                            .footer(|f| {
                                f.text(format!(
                                    "Page {} of {}",
                                    page + 1,
                                    len / queue_page_size + 1
                                ))
                            })
                            .color(config::colors::queue())
                    });

                    m
                })
                .await;
        }
        Err(err) => message_ctx.send_error(err).await,
    }

    Ok(())
}

#[command]
#[aliases(np)]
#[only_in(guilds)]
async fn now_playing(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };
    let res = GLOBAL_MEDIA_PLAYER.now_playing(guild_id).await;

    match res {
        Ok(res_tuple) => {
            match res_tuple {
                Some((info, time)) => {
                    message_ctx
                        .send_message(|m| {
                            m.content("")
                                .embed(|e| {
                                    e.title(&info.title)
                                        .description(format!(
                                            "`{} ({}/{})`",
                                            create_progress_bar(
                                                guild_id,
                                                time as f32 / info.duration as f32,
                                            ),
                                            format_timestamp(time),
                                            format_timestamp(info.duration)
                                        ))
                                        .author(|a| a.name("Now playing:"))
                                        .url(&info.url)
                                        .thumbnail(info.thumbnail)
                                        .color(config::colors::now_playing())
                                })
                                .reference_message(msg);

                            m
                        })
                        .await;
                }
                None => message_ctx.reply_error(&msg, "No songs playing!").await,
            };
        }
        Err(err) => message_ctx.reply_error(&msg, err).await,
    }

    Ok(())
}

#[command]
#[aliases(timestamps)]
#[only_in(guilds)]
async fn timestamp(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };
    let np = GLOBAL_MEDIA_PLAYER.now_playing(guild_id).await;

    match np {
        Ok(result) => {
            if let Some((song, _)) = result {
                let timestamps = parse_timestamps(song.description);

                message_ctx
                    .reply_info(
                        msg,
                        format!(
                            "{}",
                            timestamps
                                .into_iter()
                                .map(|t| format!("**{}** {}", t.timestamp, t.label))
                                .collect::<Vec<String>>()
                                .join("\n")
                        ),
                    )
                    .await;
            } else {
                message_ctx.reply_error(msg, "No song playing!").await;
            }
        }
        Err(err) => message_ctx.reply_error(msg, format!("{}", err)).await,
    }

    Ok(())
}

#[command]
async fn history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let message_ctx = MessageContext::new(ctx, msg);

    let page = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let db_plugin = get_db_plugin(ctx).await.unwrap().clone();

    let guild_id = msg.guild(&ctx.cache).unwrap().id;

    let page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    if let Ok((history, count)) =
        db_plugin.get_history(*msg.author.id.as_u64(), page_size, page * page_size)
    {
        let mut description = String::new();

        for (i, info) in history.iter().enumerate() {
            description += &format!(
                "{}. **{}**  [↗️]({})\n",
                i + 1,
                escape_string(&limit_string_length(&info.title, queue_text_len)),
                info.url
            )
            .to_string();
        }
        message_ctx
            .send_message(|m| {
                m.content("").embed(|e| {
                    e.title(format!("{}'s History", msg.author.name))
                        .description(
                            history
                                .into_iter()
                                .enumerate()
                                .map(|(i, info)| {
                                    format!(
                                        "**{}) [{}]({})** ({})",
                                        i + 1 + page * page_size,
                                        escape_string(&limit_string_length(
                                            &info.title,
                                            queue_text_len,
                                        )),
                                        info.url,
                                        format_timestamp(info.duration)
                                    )
                                })
                                .collect::<Vec<String>>()
                                .join("\n"),
                        )
                        .footer(|f| {
                            f.text(format!(
                                "Page {} of {}",
                                page + 1,
                                (count as f32 / page_size as f32).ceil()
                            ))
                        })
                        .color(config::colors::history())
                });

                m
            })
            .await;
    } else {
        message_ctx
            .send_error("Database error, unable to fetch history.")
            .await;
    }

    Ok(())
}

#[command]
#[aliases("play-history")]
#[only_in(guilds)]
async fn play_history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let message_ctx = MessageContext::new(ctx, msg);

    let no = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let db_plugin = get_db_plugin(ctx).await.unwrap().clone();

    if let Ok((history, count)) = db_plugin.get_history(*msg.author.id.as_u64(), 1, no) {
        if history.len() > 0 {
            return play(
                ctx,
                msg,
                Args::new(&history[0].url, &[Delimiter::Single(' ')]),
            )
            .await;
        } else {
            message_ctx
                .reply_warn(
                    msg,
                    format!("Song index not found. History contains {} songs.", count),
                )
                .await;
        }
    } else {
        message_ctx.reply_error(msg, "Unable to load history").await;
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
