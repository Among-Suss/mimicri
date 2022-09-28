use std::sync::Arc;

use serenity::{model::prelude::{GuildId, ChannelId}, http::Http};

use songbird::{
    input::{Input, Restartable},
    Songbird,
};

use crate::{GuildMediaPlayerMap, media};

pub async fn queue_search(
    manager: Arc<Songbird>,
    guild_id: GuildId,
    query: String,
    guild_media_player_map: &GuildMediaPlayerMap,
) -> Result<(), &'static str> {
    todo!("queue_search NOT IMPLEMENTED");

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let restartable = match Restartable::ytdl_search(query, false).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                return Err("Error sourcing ffmpeg");
            }
        };

        handler.play_only_source(Input::from(restartable));
    } else {
        return Err("Not in a channel");
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

    let mut guild_map_guard = guild_media_player_map.lock().await;
    let guild_map = guild_map_guard.as_mut().unwrap();

    if let Some(media_player) = guild_map.get(&guild_id) {
        media::media_player_enqueue(url, request_msg_channel, request_msg_http, media_player.clone()).await;
    } else {
        return Err("Not connected to a voice channel!");
    }

    Ok(())
}
