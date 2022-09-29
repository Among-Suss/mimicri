use std::sync::Arc;

use serenity::{
    http::Http,
    model::prelude::{ChannelId, GuildId},
};

use crate::{
    media::{GlobalMediaPlayer, MediaInfo, MessageContext},
    metadata::{get_info, get_search},
};

pub async fn queue_search(
    guild_id: GuildId,
    query: String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, &'static str> {
    let video = match get_search(query) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    queue_song(video.clone(), guild_id, message_ctx, global_media_player).await?;

    Ok(video)
}

pub async fn queue_url(
    guild_id: GuildId,
    url: String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, &'static str> {
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
) -> Result<(), &'static str> {
    global_media_player
        .enqueue(guild_id, info, message_ctx)
        .await
}
