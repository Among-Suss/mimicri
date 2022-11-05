use std::cmp;

use serenity::{
    framework::standard::{Args, CommandResult},
    model::prelude::{GuildId, Message},
    prelude::Context,
};
use tracing::warn;

use crate::{
    controls::commands::join,
    database::plugin::get_db_plugin,
    utils::{config, message_context::MessageContext, strings},
};

use super::metadata;
use super::{global_media_player::GlobalMediaPlayer, media_info::MediaInfo};

// Write commands

pub async fn play_command(
    media_player: &GlobalMediaPlayer,
    ctx: &Context,
    msg: &Message,
    args: Args,
    allow_playlist: bool,
) -> CommandResult {
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
            match join(media_player, ctx, msg).await {
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

    match queue_variant(
        guild_id,
        &url,
        message_ctx.clone(),
        media_player,
        allow_playlist,
    )
    .await
    {
        Ok(infos) => {
            let count = infos.len();

            if count == 1 {
                // Single song
                let info = infos.into_iter().nth(0).unwrap();

                message_ctx
                    .send_message(|m| {
                        MessageContext::format_reply(m, msg, false)
                            .content("")
                            .embed(|e| {
                                e.title(&info.title)
                                    .description(format!(
                                        "**{}**",
                                        if !info.uploader.is_empty() {
                                            info.uploader.clone()
                                        } else {
                                            "unknown".to_string()
                                        },
                                    ))
                                    .author(|a| a.name("Queued song"))
                                    .thumbnail(&info.thumbnail)
                                    .url(&info.url)
                                    .color(config::colors::play())
                            });

                        m
                    })
                    .await;

                let _ = db_plugin.set_history(*msg.author.id.as_u64(), info);
            } else if count > 1 {
                // Playlist
                let info = infos.into_iter().nth(0).unwrap();

                let playlist_info = match info.playlist {
                    Some(playlist) => (playlist.title, playlist.uploader),
                    None => (info.title, info.uploader),
                };

                message_ctx
                    .send_message(|m| {
                        MessageContext::format_reply(m, msg, false)
                            .content("")
                            .embed(|e| {
                                e.title(&playlist_info.0)
                                    .description(format!(
                                        "Uploader: **{}**\nTracks: **{}**",
                                        if !playlist_info.1.is_empty() {
                                            playlist_info.1
                                        } else {
                                            "unknown".to_string()
                                        },
                                        count
                                    ))
                                    .author(|a| a.name("Queued playlist"))
                                    .thumbnail(&info.thumbnail)
                                    .url(&info.url)
                                    .color(config::colors::play())
                            });

                        m
                    })
                    .await;
            };
        }
        Err(err) => message_ctx.reply_error(msg, err).await,
    }

    Ok(())
}

async fn queue_variant(
    guild_id: GuildId,
    query: &String,
    message_ctx: MessageContext,
    media_player: &GlobalMediaPlayer,
    allow_playlists: bool,
) -> Result<Vec<MediaInfo>, String> {
    if allow_playlists && metadata::is_playlist(query) {
        let infos = match metadata::get_playlist(query) {
            Ok(infos) => infos,
            Err(err) => return Err(err),
        };

        if infos.len() == 0 {
            return Err("Playlist is empty!".to_string());
        }

        media_player
            .enqueue_batch(guild_id, infos.clone(), message_ctx)
            .await?;

        Ok(infos.into_iter().collect::<Vec<MediaInfo>>())
    } else {
        let info = if query.starts_with("http") {
            match metadata::get_info(query) {
                Ok(url) => url,
                Err(err) => return Err(err),
            }
        } else {
            match metadata::get_search(query) {
                Ok(url) => url,
                Err(err) => return Err(err),
            }
        };

        media_player
            .enqueue(guild_id, info.clone(), message_ctx)
            .await?;

        Ok(vec![info])
    }
}

pub async fn skip(media_player: &GlobalMediaPlayer, ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let res = media_player.skip(guild_id).await;

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

pub async fn seek(
    media_player: &GlobalMediaPlayer,
    ctx: &Context,
    msg: &Message,
    args: Args,
) -> CommandResult {
    let message_context = MessageContext::new(ctx, msg);

    let arg_1 = args.raw().nth(0).unwrap_or_default().to_string();
    let guild_id = msg.guild_id.unwrap();

    let time = if let Ok(seconds) = arg_1.parse::<i64>() {
        seconds
    } else if strings::is_timestamp(&arg_1) {
        strings::parse_timestamp(&arg_1)
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

    match media_player.seek(guild_id, time).await {
        Ok(_) => {
            message_context
                .reply_info(
                    msg,
                    format!("Seeking to {}", strings::format_timestamp(time)),
                )
                .await;
        }
        Err(err) => {
            message_context.reply_error(msg, &err).await;
            warn!("Seek error: {}", &err);
        }
    }

    Ok(())
}

// Read commands

pub async fn queue(
    media_player: &GlobalMediaPlayer,
    ctx: &Context,
    msg: &Message,
    mut args: Args,
) -> CommandResult {
    let page = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    let queue_page_size = config::queue::page_size(guild_id);

    let res = media_player
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
                        MessageContext::format_embed_playlist(e, queue.iter(), len, guild_id, page)
                            .title("Queue")
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

pub async fn now_playing(
    media_player: &GlobalMediaPlayer,
    ctx: &Context,
    msg: &Message,
) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };
    let res = media_player.now_playing(guild_id).await;

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
                                            strings::create_progress_bar(
                                                guild_id,
                                                time as f32 / info.duration as f32,
                                            ),
                                            strings::format_timestamp(time),
                                            strings::format_timestamp(info.duration)
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

pub async fn timestamp(
    media_player: &GlobalMediaPlayer,
    ctx: &Context,
    msg: &Message,
) -> CommandResult {
    let guild = msg.guild(&ctx.cache).unwrap();
    let guild_id = guild.id;

    let message_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };
    let np = media_player.now_playing(guild_id).await;

    match np {
        Ok(result) => {
            if let Some((song, _)) = result {
                let timestamps = strings::parse_description_timestamps(song.description);

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
