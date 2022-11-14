use poise::command;
use serenity::model::prelude::GuildId;
use tracing::{error, warn};

use crate::{
    controls::join_channel,
    database::plugin::get_db_plugin,
    media,
    utils::{
        self, config,
        message_context::MessageContext,
        responses::{self, Responses},
        strings,
    },
    CommandResult, Context,
};

use super::{global_media_player::GlobalMediaPlayer, media_info::MediaInfo};
use super::{metadata, plugin::get_media_player};

// Write commands
#[command(
    slash_command,
    prefix_command,
    aliases("p"),
    broadcast_typing,
    category = "media"
)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "Query or url"]
    #[rest]
    song: String,
) -> CommandResult {
    media::commands::play_command(ctx, &song, true).await
}

#[command(
    slash_command,
    prefix_command,
    rename = "play-single",
    aliases("ps"),
    broadcast_typing,
    category = "media"
)]
pub async fn play_single(
    ctx: Context<'_>,
    #[description = "Query or url"] song: Vec<String>,
) -> CommandResult {
    media::commands::play_command(ctx, &song.join(" "), false).await
}

pub async fn check_or_join_vc(ctx: Context<'_>) -> Result<(), String> {
    let guild = ctx.guild().unwrap();

    let author_id = ctx.author().id;

    // Join vc
    let user_vc = match guild
        .voice_states
        .get(&author_id)
        .and_then(|voice_state| voice_state.channel_id)
    {
        Some(vc) => vc,
        None => {
            ctx.error("Ur not even in a vc idio").await;
            return Err("User not in vc".to_string());
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
                return Err("Bot already in a different channel".to_string());
            }
        }
        None => {
            match join_channel(&get_media_player(ctx.discord()).await.unwrap(), ctx).await {
                Ok(_) => (),
                Err(err) => {
                    ctx.error(&err).await;
                    error!("{:?}", &err);
                    return Err(format!("Unable to join channel: {:?}", &err));
                }
            };
        }
    };

    Ok(())
}

pub async fn play_command(ctx: Context<'_>, url: &String, allow_playlist: bool) -> CommandResult {
    ctx.defer_ephemeral()
        .await
        .expect("Failed to defer message");

    let media_player = get_media_player(ctx.discord()).await.unwrap();

    let guild = ctx.guild().unwrap();

    let author_id = ctx.author().id;

    let message_ctx = MessageContext::from(ctx);

    check_or_join_vc(ctx).await?;

    // Get url
    if url.eq("") {
        ctx.error("You didn't send anything dumbass").await;

        return Ok(());
    }

    let db_plugin = get_db_plugin(ctx.discord()).await.unwrap().clone();

    match queue_variant(guild.id, &url, message_ctx, &media_player, allow_playlist).await {
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

                response::playlist_response(
                    ctx,
                    &playlist_info.0,
                    &playlist_info.1,
                    count,
                    &info.thumbnail,
                    &info.url,
                )
                .await;
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
        let info = if strings::is_url(query) {
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

pub mod response {
    use super::*;

    pub async fn playlist_response(
        ctx: Context<'_>,
        title: &String,
        uploader: &String,
        count: usize,
        thumbnail: &String,
        url: &String,
    ) {
        ctx.send(|m| {
            m.content("").embed(|e| {
                e.title(&title)
                    .description(format!(
                        "Uploader: **{}**\nTracks: **{}**",
                        if !uploader.is_empty() {
                            uploader.as_str()
                        } else {
                            &"unknown"
                        },
                        count
                    ))
                    .author(|a| a.name("Queued playlist"))
                    .thumbnail(&thumbnail)
                    .url(&url)
                    .color(config::colors::play())
            });

            m
        })
        .await
        .expect("Failed to send message");
    }
}

#[command(slash_command, prefix_command, broadcast_typing, category = "media")]
pub async fn skip(ctx: Context<'_>) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let res = get_media_player(ctx.discord())
        .await
        .unwrap()
        .skip(guild_id)
        .await;

    match res {
        Ok(_) => ctx.info("Skipped current song!").await,
        Err(err) => ctx.error(err).await,
    }

    Ok(())
}

#[command(slash_command, prefix_command, category = "media")]
pub async fn seek(ctx: Context<'_>, to: String) -> CommandResult {
    let media_player = get_media_player(ctx.discord()).await.unwrap();

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
#[command(slash_command, prefix_command, category = "media")]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    let Some(page) = utils::validate_page(ctx, page).await else {
        return Ok(());
    };

    let media_player = get_media_player(ctx.discord()).await.unwrap();

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

#[command(
    slash_command,
    prefix_command,
    rename = "now-playing",
    aliases("np"),
    category = "media"
)]
pub async fn now_playing(ctx: Context<'_>) -> CommandResult {
    let media_player = get_media_player(ctx.discord()).await.unwrap();

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

#[command(slash_command, prefix_command, category = "media")]
pub async fn timestamp(ctx: Context<'_>) -> CommandResult {
    let media_player = get_media_player(ctx.discord()).await.unwrap();

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
