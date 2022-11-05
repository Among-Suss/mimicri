use serenity::{
    framework::standard::{Args, CommandResult},
    model::prelude::{GuildId, Message},
    prelude::Context,
};

use crate::{
    controls::commands::join,
    database::plugin::get_db_plugin,
    utils::{config, message_context::MessageContext},
};

use super::metadata;
use super::{global_media_player::GlobalMediaPlayer, media_info::MediaInfo};

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
                let info = infos.into_iter().nth(0).unwrap();

                message_ctx
                    .send_message(|m| {
                        m.content("")
                            .embed(|e| {
                                e.title(&info.title)
                                    .description(format!("**{}**", info.uploader))
                                    .author(|a| a.name("Queued song"))
                                    .thumbnail(&info.thumbnail)
                                    .url(&info.url)
                                    .color(config::colors::play())
                            })
                            .reference_message(msg);

                        m
                    })
                    .await;

                let _ = db_plugin.set_history(*msg.author.id.as_u64(), info);
            } else if count > 1 {
                let info = infos.into_iter().nth(0).unwrap();

                let playlist_info = match info.playlist {
                    Some(playlist) => (playlist.title, playlist.uploader),
                    None => (info.title, info.uploader),
                };

                message_ctx
                    .send_message(|m| {
                        m.content("")
                            .embed(|e| {
                                e.title(&playlist_info.0)
                                    .description(format!(
                                        "Uploader: **{}**\nTracks: **{}**",
                                        playlist_info.1, count
                                    ))
                                    .author(|a| a.name("Queued playlist"))
                                    .thumbnail(&info.thumbnail)
                                    .url(&info.url)
                                    .color(config::colors::play())
                            })
                            .reference_message(msg);

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
