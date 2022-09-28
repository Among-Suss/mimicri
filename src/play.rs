use std::sync::Arc;

use serenity::{
    http::Http,
    model::prelude::{ChannelId, GuildId},
};

use crate::{
    media,
    metadata::{get_info, get_search},
    GuildMediaPlayerMap,
};

pub async fn queue_search(
    guild_id: GuildId,
    query: String,
    request_msg_channel: ChannelId,
    request_msg_http: Arc<Http>,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<(), &'static str> {
    let video = match get_search(query) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    let mut guild_map_guard = guild_media_player_map.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();

    if let Some(media_player) = guild_map.get(&guild_id) {
        media::media_player_enqueue(video, request_msg_channel, request_msg_http, &media_player)
            .await;
    } else {
        return Err("Not connected to a voice channel!");
    }

    Ok(())
}

pub async fn queue_url(
    guild_id: GuildId,
    url: String,
    request_msg_channel: ChannelId,
    request_msg_http: Arc<Http>,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<(), &'static str> {
    let video = match get_info(url) {
        Ok(url) => url,
        Err(err) => return Err(err),
    };

    let mut guild_map_guard = guild_media_player_map.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();

    if let Some(media_player) = guild_map.get(&guild_id) {
        media::media_player_enqueue(video, request_msg_channel, request_msg_http, &media_player)
            .await;
    } else {
        return Err("Not connected to a voice channel!");
    }

    Ok(())
}
