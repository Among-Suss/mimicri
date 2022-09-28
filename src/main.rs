//! Requires the "client", "standard_framework", and "voice" features be enabled in your
//! Cargo.toml, like so:
//!
//! ```toml
//! [dependencies.serenity]
//! git = "https://github.com/serenity-rs/serenity.git"
//! features = ["client", "standard_framework", "voice"]
//! ```
mod media;
mod metadata;
mod play;

use dotenv::dotenv;
use media::ChannelMediaPlayer;
use std::{env, collections::{HashMap, LinkedList}, sync::Arc};

// This trait adds the `register_songbird` and `register_songbird_with` methods
// to the client builder below, making it easy to install this voice client.
// The voice client can be retrieved in any command using `songbird::get(ctx).await`.
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

use crate::play::{queue_search, queue_url};

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[group]
#[commands(deafen, join, leave, mute, play, ping, skip, undeafen, unmute)]
struct General;

type GuildMediaPlayerMap = async_std::sync::Mutex<Option<HashMap<serenity::model::prelude::GuildId, Arc<ChannelMediaPlayer>>>>;

static GUILD_MEDIA_PLAYER_MAP: GuildMediaPlayerMap = async_std::sync::Mutex::new(None);


#[tokio::main]
async fn main() {
    {
        let mut guild_map = GUILD_MEDIA_PLAYER_MAP.lock().await;
        match &*guild_map {
            Some(_) => panic!("HashMap should be uninitialized!"),
            None => *guild_map = Some(
                HashMap::new()
            ),
        };
    }

    tracing_subscriber::fmt::init();

    // Configure the client with your Discord bot token in the environment.
    dotenv().ok();

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let prefix = env::var("BOT_PREFIX").unwrap_or("~".to_owned());

    let framework = StandardFramework::new()
        .configure(|c| c.prefix(prefix))
        .group(&GENERAL_GROUP);

    let intents = GatewayIntents::non_privileged() | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .register_songbird()
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
    let url = args.raw().collect::<Vec<&str>>().join(", ");

    if url.eq("") {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "You didn't send anything, dumbass")
                .await,
        );

        return Ok(());
    }

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if url.starts_with("http") {
        match queue_url(guild_id, url, msg.channel_id, ctx.http.clone(), &GUILD_MEDIA_PLAYER_MAP).await {
            Ok(_) => (),
            Err(err) => {
                check_msg(msg.channel_id.say(&ctx.http, err).await);
            }
        }
    } else {
        match queue_search(manager, guild_id, url, &GUILD_MEDIA_PLAYER_MAP).await {
            Ok(_) => (),
            Err(err) => {
                check_msg(msg.channel_id.say(&ctx.http, err).await);
            }
        }
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

    let mut guild_map_guard = GUILD_MEDIA_PLAYER_MAP.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();
    if guild_map.contains_key(&guild_id) {
        check_msg(msg.reply(ctx, "Already connected to a voice channel in this server!").await);
    } else {

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

        let media_player = Arc::new(ChannelMediaPlayer {
            guild_id,
            lock_protected_media_queue: (
                async_std::sync::Mutex::new(media::MediaQueue { now_playing: None, queue: LinkedList::new() }),
                async_std::sync::Condvar::new(),
            ),
        });

        let _ = guild_map.insert(guild_id, media_player.clone());
        tokio::spawn(media::media_player_run(handler.0, media_player));
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
async fn skip(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let mut guild_map_guard = GUILD_MEDIA_PLAYER_MAP.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();

    if let Some(media_player) = guild_map.get(&guild_id) {
        media::media_player_skip(media_player.clone()).await;
    } else {
        check_msg(msg.reply(ctx, "Not connected to a voice channel!").await);
    }

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
