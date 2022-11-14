use std::{collections::LinkedList, sync::Arc};

use poise::{command, serenity_prelude::CreateEmbed};
use tracing::error;

use crate::{
    media::{self, media_info::MediaInfo, metadata},
    utils::{
        self, config,
        responses::{self, Responses},
        strings, validate_page,
    },
    CommandResult, Context,
};

use super::plugin::{get_db_plugin, DatabasePlugin};

// Playlists

#[command(
    slash_command,
    prefix_command,
    category = "playlists",
    subcommands("list_playlists", "create_playlist", "play_playlist", "show_playlist")
)]
pub async fn playlists(ctx: Context<'_>, #[min = 1] page: Option<i64>) -> CommandResult {
    _list_playlists(ctx, page).await
}

/// Lists your playlists
#[command(slash_command, prefix_command, rename = "list", category = "playlists")]
pub async fn list_playlists(ctx: Context<'_>, #[min = 1] page: Option<i64>) -> CommandResult {
    _list_playlists(ctx, page).await
}

async fn _list_playlists(ctx: Context<'_>, page: Option<i64>) -> CommandResult {
    let page = validate_page(ctx, page).await?;

    let page_size = config::queue::page_size(ctx.guild_id().unwrap());

    responses::create_pagination(ctx, page, |next_page| async move {
        let db = get_db(ctx).await?;

        let (playlists, count) = db
            .get_playlists(ctx.author().id, page_size, page * page_size)
            .expect("DB Plugin disabled"); // FIXME I'm lazy

        Ok((
            response::print_playlists(
                &mut CreateEmbed::default(),
                &format!("{}'s playlists", ctx.author().name).to_string(),
                playlists,
                page,
                count,
                page_size,
            )
            .to_owned(),
            next_page,
            utils::ceil(count, page_size),
        ))
    })
    .await?;

    Ok(())
}

/// Create a playlist
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

/// Queue all tracks from a playlist
#[command(slash_command, prefix_command, rename = "play", category = "playlists")]
async fn play_playlist(
    ctx: Context<'_>,
    #[description = "Playlist"]
    #[rename = "playlist"]
    #[rest]
    #[autocomplete = "autocomplete_playlists"]
    playlist_name: String,
) -> CommandResult {
    let db = get_db(ctx).await?;
    let media_playlist = media::plugin::get_media_player(ctx.discord())
        .await
        .unwrap();

    let Ok(playlist) = db.get_playlist(ctx.author().id, &playlist_name, 9999, 0) else {
        ctx.error("Unable to retreive songs from playlist").await;
        return Ok(())
    };

    let songs = playlist.0;

    media::commands::check_or_join_vc(ctx).await?;

    media_playlist
        .enqueue_batch(
            ctx.guild_id().unwrap(),
            songs.into_iter().collect::<LinkedList<MediaInfo>>(),
            ctx.into(),
        )
        .await?;

    media::commands::response::playlist_response(
        ctx,
        &playlist_name,
        &ctx.author().name,
        playlist.1,
        &"".to_string(),
        &"".to_string(),
    )
    .await;

    Ok(())
}

/// View tracks of a playlist
#[command(
    slash_command,
    prefix_command,
    rename = "tracks",
    category = "playlists"
)]
pub async fn show_playlist(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_playlists"] playlist: String,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    let initial_page = validate_page(ctx, page).await?;

    let playlist_name = &playlist.clone();

    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let page_size = config::queue::page_size(guild_id);

    responses::create_pagination(ctx, initial_page, |next_page| async move {
        let db = get_db(ctx).await?;

        let res = db.get_playlist(
            ctx.author().id,
            playlist_name,
            page_size,
            next_page * page_size,
        );

        match res {
            Ok((queue, len)) => {
                if len == 0 {
                    Err("The queue is empty".to_string())
                } else {
                    Ok((
                        responses::format_embed_playlist(
                            &mut CreateEmbed::default(),
                            queue.iter(),
                            len,
                            guild_id,
                            next_page,
                        )
                        .title(format!("{} ({})", playlist_name, ctx.author().name))
                        .color(config::colors::playlist())
                        .to_owned(),
                        next_page,
                        (len + page_size - 1) / page_size,
                    ))
                }
            }
            Err(err) => Err(format!("{}", err)),
        }
    })
    .await?;

    Ok(())
}

async fn autocomplete_playlists<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let Some(db) = get_db_plugin(ctx.discord()).await else {
        return vec![].into_iter();
    };

    db.search_playlists(ctx.author().id, &partial.to_string())
        .unwrap()
        .into_iter()
}

// History
#[command(
    slash_command,
    prefix_command,
    category = "playlists",
    subcommands("list_history", "queue_history")
)]
pub async fn history(
    ctx: Context<'_>,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    _list_history(ctx, page).await
}

/// Show your history
#[command(slash_command, prefix_command, rename = "show", category = "playlists")]
async fn list_history(
    ctx: Context<'_>,
    #[description = "Page #"]
    #[min = 1]
    page: Option<i64>,
) -> CommandResult {
    _list_history(ctx, page).await
}

async fn _list_history(ctx: Context<'_>, page: Option<i64>) -> CommandResult {
    let initial_page = validate_page(ctx, page).await?;

    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let page_size = config::queue::page_size(guild_id);

    responses::create_pagination(ctx, initial_page, |next_page| async move {
        let db = get_db(ctx).await?;

        let res = db.get_history(ctx.author().id, page_size, next_page * page_size);

        match res {
            Ok((queue, len)) => {
                if len == 0 {
                    Err("The queue is empty".to_string())
                } else {
                    Ok((
                        responses::format_embed_playlist(
                            &mut CreateEmbed::default(),
                            queue.iter(),
                            len,
                            guild_id,
                            next_page,
                        )
                        .title(format!("{}'s history", ctx.author().name))
                        .color(config::colors::history())
                        .to_owned(),
                        next_page,
                        (len + page_size - 1) / page_size,
                    ))
                }
            }
            Err(err) => Err(format!("{}", err)),
        }
    })
    .await?;

    Ok(())
}

/// Queue a song from your history
#[command(
    slash_command,
    prefix_command,
    rename = "queue",
    category = "playlists"
)]
async fn queue_history(
    ctx: Context<'_>,
    #[description = "Index #"]
    #[min = 1]
    index: i64,
) -> CommandResult {
    if index < 1 {
        ctx.error("Index cannot be less than 1").await;
        return Ok(());
    }

    let db = get_db(ctx).await?;

    if let Ok((history, count)) = db.get_history(ctx.author().id, 1, (index - 1) as usize) {
        if history.len() > 0 {
            media::commands::play_command(ctx, &history[0].url.clone(), false).await?;
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

    Ok(())
}

// Helpers

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

mod response {
    use super::*;

    pub fn print_playlists<'a>(
        e: &'a mut CreateEmbed,
        title: &String,
        playlists: Vec<String>,
        page: usize,
        total: usize,
        page_size: usize,
    ) -> &'a mut CreateEmbed {
        e.title(format!("{}", title))
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
                m.text(strings::page_display(
                    page + 1,
                    total,
                    page_size,
                    &"playlist",
                ))
            })
            .color(config::colors::playlist())
    }
}
