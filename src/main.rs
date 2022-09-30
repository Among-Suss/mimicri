mod database_plugin;
mod media;
mod metadata;
mod play;
mod strings;

use dotenv::dotenv;
use media::GlobalMediaPlayer;
use std::{env, sync::Arc};

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
    media::MessageContext,
    play::queue_url_or_search,
    strings::{escape_string, limit_string_length},
};

static QUEUE_TEXT_LENGTH: usize = 75;
static QUEUE_PAGE_SIZE: usize = 10;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(deafen, join, leave, mute, play, ping, skip, queue, undeafen, unmute)]
struct General;

static GLOBAL_MEDIA_PLAYER: GlobalMediaPlayer = GlobalMediaPlayer::UNINITIALIZED;

#[tokio::main]
async fn main() {
    GLOBAL_MEDIA_PLAYER.init_self().await;

    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
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
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    // Join vc
    let user_vc = match guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(vc) => vc,
        None => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Ur not even in a vc idio")
                    .await,
            );
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
                check_msg(msg.channel_id.say(&ctx.http, "Wrong channel dumbass").await);
                return Ok(());
            }
        }
        None => {
            match join(ctx, msg, args.clone()).await {
                Ok(_) => (),
                Err(err) => check_msg(msg.channel_id.say(&ctx.http, err).await),
            };
        }
    }

    // Get url
    let url = args.raw().collect::<Vec<&str>>().join(" ");

    if url.eq("") {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "You didn't send anything, dumbass")
                .await,
        );

        return Ok(());
    }

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    let db_plugin = database_plugin::plugin::get(ctx).await.unwrap().clone();

    match queue_url_or_search(guild_id, &url, message_ctx, &GLOBAL_MEDIA_PLAYER).await {
        Ok(info) => {
            check_msg(
                msg.channel_id
                    .send_message(&ctx.http, |m| {
                        m.content("Queued song:")
                            .embed(|e| e.title(info.title).description(&info.url))
                    })
                    .await,
            );

            let _ = db_plugin.set_history(*ctx.cache.current_user_id().as_u64() as i64, &info.url);
        }
        Err(err) => {
            check_msg(msg.channel_id.say(&ctx.http, err).await);
        }
    }

    Ok(())
}

#[command]
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let res = GLOBAL_MEDIA_PLAYER.skip(guild_id).await;

    match res {
        Ok(_) => check_msg(
            msg.channel_id
                .send_message(&ctx.http, |m| m.embed(|e| e.title("Skipped current song!")))
                .await,
        ),
        Err(err) => check_msg(msg.channel_id.say(&ctx.http, err).await),
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut page = str::parse::<usize>(args.raw().nth(0).unwrap_or_default()).unwrap_or_default();
    page = if page > 0 { page - 1 } else { 0 };

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let res = GLOBAL_MEDIA_PLAYER
        .read_queue(guild_id, page * QUEUE_PAGE_SIZE, QUEUE_PAGE_SIZE)
        .await;

    match res {
        Ok((queue, len)) => {
            if len == 0 {
                // TODO empty queue command
                return Ok(());
            }

            let mut str = String::new();

            for (i, info) in queue.iter().enumerate() {
                str += &format!(
                    "{}. **{}**  [↗️]({})\n",
                    i + 1 + page * QUEUE_PAGE_SIZE,
                    escape_string(&limit_string_length(&info.title, QUEUE_TEXT_LENGTH)),
                    info.url
                )
                .to_string();
            }

            str += &format!(
                "\n...\n\nPage {} of {}",
                page + 1,
                len / QUEUE_PAGE_SIZE + 1
            );

            check_msg(
                msg.channel_id
                    .send_message(&ctx.http, |m| {
                        m.embed(|e| e.title("Queue").description(&str))
                    })
                    .await,
            );
        }
        Err(err) => check_msg(msg.channel_id.say(&ctx.http, err).await),
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
