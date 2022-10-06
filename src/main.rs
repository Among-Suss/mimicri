mod config;
mod database_plugin;
mod media;
mod message_context;
mod metadata;
mod play;
mod strings;

use dotenv::dotenv;
use media::GlobalMediaPlayer;
use std::{cmp, env, sync::Arc};

use songbird::SerenityInit;

use serenity::{
    async_trait,
    client::{Client, Context, EventHandler},
    framework::{
        standard::{
            macros::{command, group},
            Args, CommandResult,
        },
        StandardFramework,
    },
    model::{channel::Message, gateway::Ready},
    prelude::GatewayIntents,
    Result as SerenityResult,
};

use crate::{
    database_plugin::{plugin::DatabasePluginInit, sqlite_plugin::SQLitePlugin},
    message_context::MessageContext,
    play::queue_variant,
    strings::{create_progress_bar, escape_string, limit_string_length, ProgressBarStyle},
};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(
    version,
    play,
    play_single,
    ping,
    skip,
    queue,
    undeafen,
    unmute,
    now_playing,
    deafen,
    join,
    leave,
    mute
)]
struct General;

static GLOBAL_MEDIA_PLAYER: GlobalMediaPlayer = GlobalMediaPlayer::UNINITIALIZED;

#[tokio::main]
async fn main() {
    GLOBAL_MEDIA_PLAYER.init_self().await;

    tracing_subscriber::fmt::init();

    dotenv().ok();

    let token = match env::var("DISCORD_TOKEN") {
        Ok(var) => var,
        Err(_) => {
            println!("[Warning] No DISCORD_TOKEN environment variable present. Have you set the correct environment variables?\n\tSee the README for a list of available environment variables.");
            return;
        }
    };

    let prefix = env::var("BOT_PREFIX").unwrap_or("~".to_owned());

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(prefix))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
        .register_database_plugin(Arc::new(SQLitePlugin::default()))
        .await
        .expect("Err creating client");

    tokio::spawn(async move {
        let _ = client
            .start()
            .await
            .map_err(|why| println!("Client ended: {:?}", why));
    });

    tokio::signal::ctrl_c()
        .await
        .expect("Failed to shutdown correctly");
    println!("Received Ctrl-C, shutting down.");
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
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

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

    let db_plugin = database_plugin::plugin::get(ctx).await.unwrap().clone();

    match queue_variant(guild_id, &url, message_ctx.clone(), &GLOBAL_MEDIA_PLAYER).await {
        Ok(info) => {
            message_ctx
                .reply_embed(msg, "Queued song:", &info.title, &info.url)
                .await;

            let _ = db_plugin.set_history(*ctx.cache.current_user_id().as_u64() as i64, &info.url);
        }
        Err(err) => message_ctx.reply_error(msg, err).await,
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn play_single(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let list_index = msg.content.find("&list=");

    let mut new_msg = msg.clone();

    if let Some(ind) = list_index {
        new_msg.content = msg.content.clone()[0..ind].to_string();
    }

    play(ctx, &new_msg, args);

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
        Ok(_) => message_context.send_info("Skipped current song!").await,
        Err(err) => message_context.send_info(err).await,
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut page = str::parse::<usize>(args.raw().nth(0).unwrap_or_default()).unwrap_or_default();
    page = cmp::max((page as i32) - 1, 0) as usize;

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    let queue_page_size = config::queue_page_size(guild_id);
    let queue_text_len = config::queue_text_length(guild_id);

    let res = GLOBAL_MEDIA_PLAYER
        .read_queue(guild_id, page * queue_page_size, queue_page_size)
        .await;

    match res {
        Ok((queue, len)) => {
            if len == 0 {
                // TODO empty queue command
                return Ok(());
            }

            let mut description = String::new();

            for (i, info) in queue.iter().enumerate() {
                description += &format!(
                    "{}. **{}**  [↗️]({})\n",
                    i + 1 + page * queue_page_size,
                    escape_string(&limit_string_length(&info.title, queue_text_len)),
                    info.url
                )
                .to_string();
            }

            description += &format!(
                "\n...\n\nPage {} of {}",
                page + 1,
                len / queue_page_size + 1
            );

            message_ctx.send_embed("", "Queue", description).await;
        }
        Err(err) => message_ctx.send_error(err).await,
    }

    Ok(())
}

#[command]
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
                        .reply_embed(
                            &msg,
                            "Now playing:",
                            &info.title,
                            &format!(
                                "{}\n{}",
                                &info.url,
                                create_progress_bar(
                                    guild_id,
                                    time as f32 / info.duration as f32,
                                    ProgressBarStyle::Marker
                                )
                            ),
                        )
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
#[only_in(guilds)]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        check_msg(msg.channel_id.say(&ctx.http, "Already deafened").await);
    } else {
        if let Err(e) = handler.deafen(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Deafened").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler = manager.join(guild_id, connect_to).await;

    if let Ok(_) = handler.1 {
        let res = GLOBAL_MEDIA_PLAYER.start(guild_id, handler.0).await;
        match res {
            Ok(_) => (),
            Err(err) => {
                check_msg(msg.reply(ctx, err).await);
            }
        }
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        let res = GLOBAL_MEDIA_PLAYER.quit(guild_id).await;
        match res {
            Ok(_) => (),
            Err(err) => check_msg(msg.reply(ctx, err).await),
        }

        if let Err(e) = manager.remove(guild_id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(msg.channel_id.say(&ctx.http, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Now muted").await);
    }

    Ok(())
}

#[command]
async fn ping(context: &Context, msg: &Message) -> CommandResult {
    check_msg(msg.channel_id.say(&context.http, "Pong!").await);

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Undeafened").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to undeafen in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Unmuted").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
