use serenity::model::prelude::GuildId;
use tracing::{error, warn};

use crate::{
    controls::commands::join,
    database::plugin::get_db_plugin,
    utils::{
        config,
        message_context::MessageContext,
        responses::{self, Responses},
        strings,
    },
    CommandResult, Context,
};

use super::metadata;
use super::{global_media_player::GlobalMediaPlayer, media_info::MediaInfo};

// Write commands

pub async fn play_command(
    media_player: &GlobalMediaPlayer,
    ctx: Context<'_>,
    url: &String,
    allow_playlist: bool,
) -> CommandResult {
    let guild = ctx.guild().unwrap();

    let author_id = ctx.author().id;

    let message_ctx = MessageContext::from(ctx);

    // Join vc
    let user_vc = match guild
        .voice_states
        .get(&author_id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(vc) => vc,
        None => {
            ctx.error("Ur not even in a vc idio").await;
            return Ok(());
        }
    };

    match guild
        .voice_states
        .get(&ctx.discord().cache.current_user_id())
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(bot_vc) => {
            if bot_vc != user_vc {
                ctx.error("Wrong channel dumbass").await;
                return Ok(());
            }
        }
        None => {
            match join(media_player, ctx).await {
                Ok(_) => (),
                Err(err) => {
                    ctx.error(&err).await;
                    error!("{:?}", &err);
                    return Ok(());
                }
            };
        }
    }

    // Get url
    if url.eq("") {
        ctx.error("You didn't send anything dumbass").await;

        return Ok(());
    }

    let db_plugin = get_db_plugin(ctx.discord()).await.unwrap().clone();

    match queue_variant(guild.id, &url, message_ctx, media_player, allow_playlist).await {
        Ok(infos) => {
            let count = infos.len();

            if count == 1 {
                // Single song
                let info = infos.into_iter().nth(0).unwrap();

                ctx.send(|m| {
                    m.content("").embed(|e| {
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
                    })
                })
                .await
                .expect("Failed to send message");

                let _ = db_plugin.set_history(author_id, &info);
            } else if count > 1 {
                // Playlist
                let info = infos.into_iter().nth(0).unwrap();

                let playlist_info = match info.playlist {
                    Some(playlist) => (playlist.title, playlist.uploader),
                    None => (info.title, info.uploader),
                };

                ctx.send(|m| {
                    m.content("").embed(|e| {
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
                .await
                .expect("Failed to send message");
            };
        }
        Err(err) => ctx.error(err).await,
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

pub async fn skip(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let res = media_player.skip(guild_id).await;

    match res {
        Ok(_) => ctx.info("Skipped current song!").await,
        Err(err) => ctx.error(err).await,
    }

    Ok(())
}

pub async fn seek(
    media_player: &GlobalMediaPlayer,
    ctx: Context<'_>,
    to: &String,
) -> CommandResult {
    let guild_id = ctx.guild().unwrap().id;

    let time = if let Ok(seconds) = to.parse::<i64>() {
        seconds
    } else if strings::is_timestamp(&to) {
        strings::parse_timestamp(&to)
    } else {
        ctx.error(format!("{} isn't a valid timestamp.", to)).await;

        return Ok(());
    };

    if time < 0 {
        ctx.error("Cannot seek to negative time.").await;

        return Ok(());
    }

    match media_player.seek(guild_id, time).await {
        Ok(_) => {
            ctx.info(format!("Seeking to {}", strings::format_timestamp(time)))
                .await;
        }
        Err(err) => {
            ctx.error(&err).await;
            warn!("Seek error: {}", &err);
        }
    }

    Ok(())
}

// Read commands

pub async fn queue(
    media_player: &GlobalMediaPlayer,
    ctx: Context<'_>,
    page: usize,
) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let queue_page_size = config::queue::page_size(guild_id);

    let res = media_player
        .read_queue(guild_id, page * queue_page_size, queue_page_size)
        .await;

    match res {
        Ok((queue, len)) => {
            if len == 0 {
                ctx.info("The queue is empty!").await;
                return Ok(());
            }

            ctx.send(|m| {
                m.content("").embed(|e| {
                    responses::format_embed_playlist(e, queue.iter(), len, guild_id, page)
                        .title("Queue")
                        .color(config::colors::queue())
                })
            })
            .await
            .expect("Failed to send message");
        }
        Err(err) => ctx.error(err).await,
    }

    Ok(())
}

pub async fn now_playing(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let res = media_player.now_playing(guild_id).await;

    match res {
        Ok(res_tuple) => {
            match res_tuple {
                Some((info, time)) => {
                    ctx.send(|m| {
                        m.content("").embed(|e| {
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
                    })
                    .await
                    .expect("Failed to send message");
                }
                None => ctx.error("No songs playing!").await,
            };
        }
        Err(err) => ctx.error(err).await,
    }

    Ok(())
}

pub async fn timestamp(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let np = media_player.now_playing(guild_id).await;

    match np {
        Ok(result) => {
            if let Some((song, _)) = result {
                let timestamps = strings::parse_description_timestamps(song.description);

                ctx.info(format!(
                    "{}",
                    timestamps
                        .into_iter()
                        .map(|t| format!("**{}** {}", t.timestamp, t.label))
                        .collect::<Vec<String>>()
                        .join("\n")
                ))
                .await;
            } else {
                ctx.error("No song playing!").await;
            }
        }
        Err(err) => ctx.error(format!("{}", err)).await,
    }

    Ok(())
}
