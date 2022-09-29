use std::sync::Arc;

use serenity::{
    http::Http,
    model::prelude::{ChannelId, GuildId},
};

use crate::{
    media::{self, MediaInfo},
    metadata::{get_info, get_search},
    GuildMediaPlayerMap,
};

pub async fn queue_search(
    guild_id: GuildId,
    query: String,
    request_msg_channel: ChannelId,
    request_msg_http: Arc<Http>,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<MediaInfo, &'static str> {
    let video = match get_search(query) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    queue_song(
        video.clone(),
        guild_id,
        request_msg_channel,
        request_msg_http,
        guild_media_player_map,
    )
    .await?;

    Ok(video)
}

pub async fn queue_url(
    guild_id: GuildId,
    url: String,
    request_msg_channel: ChannelId,
    request_msg_http: Arc<Http>,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<MediaInfo, &'static str> {
    let video = match get_info(url) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    queue_song(
        video.clone(),
        guild_id,
        request_msg_channel,
        request_msg_http,
        guild_media_player_map,
    )
    .await?;

    Ok(video)
}

async fn queue_song(
    info: MediaInfo,
    guild_id: GuildId,
    request_msg_channel: ChannelId,
    request_msg_http: Arc<Http>,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<(), &'static str> {
    let mut guild_map_guard = guild_media_player_map.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();

    if let Some(media_player) = guild_map.get(&guild_id) {
        media::media_player_enqueue(info, request_msg_channel, request_msg_http, &media_player)
            .await;
    } else {
        return Err("Not connected to a voice channel!");
    }

    Ok(())
}
