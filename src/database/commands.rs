use std::sync::Arc;

use poise::command;
use tracing::error;

use crate::{
    media::{media_info::MediaInfo, metadata},
    utils::{
        config,
        responses::{self, Responses},
        strings,
    },
    CommandResult, Context,
};

use super::plugin::{get_db_plugin, DatabasePlugin};

// Playlists

#[command(
    slash_command,
    prefix_command,
    category = "playlists",
    subcommands("create_playlist", "get_playlists")
)]
pub async fn playlists(_ctx: Context<'_>) -> CommandResult {
    Ok(())
}

#[command(slash_command, prefix_command, rename = "create")]
async fn create_playlist(
    ctx: Context<'_>,
    #[description = "Playlist name or URL"] playlist: String,
) -> CommandResult {
    ctx.defer().await.unwrap();

    let db = get_db(ctx).await?;

    if strings::is_url(&playlist) {
        // Url
        if metadata::is_playlist(&playlist) {
            let Ok(mut songs) = metadata::get_playlist(&playlist) else {
                ctx.error("Couldn't retreive song from playlist").await;
                error!("get_playlist error");

                return Ok(());
            };

            // Playlist
            if songs.len() <= 0 {
                ctx.error("Playlist is empty!").await;

                return Ok(());
            }

            let first_song = songs.pop_front().unwrap();
            let some_playlist_info = &first_song.playlist.clone();

            let Some(playlist_info) = some_playlist_info else {
                ctx.error("Couldn't retreive playlist data").await;
                error!("get_playlist playlist info is empty");
                return Ok(());
            };

            // Create playlist
            if let Err(err) = db.create_playlist(ctx.author().id, &playlist_info.title) {
                ctx.error("Failed to create playlist").await;
                error!("{}", err);
                return Ok(());
            }

            // Add songs to playlist
            songs.push_front(first_song);

            let len = songs.len();

            let _ = db.add_playlist_songs(
                ctx.author().id,
                &playlist_info.title,
                songs.iter().collect::<Vec<&MediaInfo>>(),
            );

            ctx.send(|m| {
                m.embed(|e| {
                    e.title(format!(
                        "Created a playlist: {} ({})",
                        playlist_info.title,
                        ctx.author().name
                    ))
                    .description(format!(
                        "Uploader: **{}**\nTracks: **{}**",
                        playlist_info.uploader, len
                    ))
                })
            })
            .await?;
        } else {
            ctx.error("URL given is not a playlist!").await;
        }
    } else {
        // Name
        if let Err(err) = db.create_playlist(ctx.author().id, &playlist) {
            ctx.error("Failed to create playlist").await;
            error!("{}", err);
        }
    }

    Ok(())
}

#[command(slash_command, prefix_command, rename = "get")]
async fn get_playlists(ctx: Context<'_>, #[min = 1] page: usize) -> CommandResult {
    let db = get_db(ctx).await?;

    let page_size = config::queue::page_size(ctx.guild_id().unwrap());

    let (playlists, count) = db
        .get_playlists(ctx.author().id, page_size, (page - 1) * page_size)
        .expect("DB Plugin disabled"); // FIXME I'm lazy

    response::get_playlist(ctx, playlists, page, count, page_size).await?;

    Ok(())
}

mod response {
    use super::*;

    pub async fn get_playlist(
        ctx: Context<'_>,
        playlists: Vec<String>,
        page: usize,
        total: usize,
        page_size: usize,
    ) -> CommandResult {
        ctx.send(|m| {
            m.embed(|e| {
                e.title(format!("{}'s playlist", ctx.author().name))
                    .description(format!(
                        "{}",
                        playlists
                            .into_iter()
                            .enumerate()
                            .map(|(i, p)| format!("{}. **{}**", i + 1, p))
                            .collect::<Vec<String>>()
                            .join("\n")
                    ))
                    .footer(|m| {
                        m.text(format!(
                            "Page {} of {}, total: {}",
                            page,
                            total / page_size + 1,
                            total
                        ))
                    })
                    .color(config::colors::playlist())
            })
        })
        .await?;

        Ok(())
    }
}

// History

pub async fn history(ctx: Context<'_>, page: usize) -> CommandResult {
    let guild_id = ctx.guild().unwrap().id;

    let page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    if let Some(db_plugin) = get_db_plugin(ctx.discord()).await {
        if let Ok((history, count)) =
            db_plugin.get_history(ctx.author().id, page_size, page * page_size)
        {
            let mut description = String::new();

            for (i, info) in history.iter().enumerate() {
                description += &format!(
                    "{}. **{}**  [↗️]({})\n",
                    i + 1,
                    strings::escape_string(&strings::limit_string_length(
                        &info.title,
                        queue_text_len
                    )),
                    info.url
                )
                .to_string();
            }
            ctx.send(|m| {
                m.content("").embed(|e| {
                    responses::format_embed_playlist(e, history.iter(), count, guild_id, page)
                        .title(format!("{}'s History", ctx.author().name))
                        .color(config::colors::history())
                })
            })
            .await
            .expect("Failed to send message");
        } else {
            ctx.error("Database error, unable to fetch history.").await;
        }
    }
    Ok(())
}

pub async fn get_history(ctx: Context<'_>, page: usize) -> Option<String> {
    if let Some(db_plugin) = get_db_plugin(ctx.discord()).await {
        if let Ok((history, count)) = db_plugin.get_history(ctx.author().id, 1, page) {
            if history.len() > 0 {
                return Some(history[0].url.clone());
            } else {
                ctx.warn(format!(
                    "Song index not found. History contains {} songs.",
                    count
                ))
                .await;
            }
        } else {
            ctx.error("Unable to load history").await;
        }
    }

    None
}

async fn get_db(ctx: Context<'_>) -> Result<Arc<dyn DatabasePlugin>, String> {
    let db = get_db_plugin(ctx.discord())
        .await
        .ok_or("Plugin not initialized!")?;

    if db.disabled() {
        ctx.error("Playlist functionalities are disabled on this bot!")
            .await;
        return Err("Playlist functionalities are disabled on this bot!".to_string());
    }

    Ok(db)
}
