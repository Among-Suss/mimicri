use serenity::model::prelude::GuildId;

use crate::{
    media::{GlobalMediaPlayer, MediaInfo, MessageContext},
    metadata::{get_info, get_search},
};

pub async fn queue_url_or_search(
    guild_id: GuildId,
    query: &String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, String> {
    if query.starts_with("http") {
        return queue_url(guild_id, query, message_ctx, global_media_player).await;
    } else {
        return queue_search(guild_id, query, message_ctx, global_media_player).await;
    }
}

pub async fn queue_search(
    guild_id: GuildId,
    query: &String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, String> {
    let video = match get_search(query) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    queue_song(video.clone(), guild_id, message_ctx, global_media_player).await?;

    Ok(video)
}

pub async fn queue_url(
    guild_id: GuildId,
    url: &String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, String> {
    let video = match get_info(url) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    queue_song(video.clone(), guild_id, message_ctx, global_media_player).await?;

    Ok(video)
}

async fn queue_song(
    info: MediaInfo,
    guild_id: GuildId,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<(), String> {
    global_media_player
        .enqueue(guild_id, info, message_ctx)
        .await
}
