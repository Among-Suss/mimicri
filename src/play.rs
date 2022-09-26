use std::sync::Arc;

use serenity::model::prelude::GuildId;

use songbird::{
    input::{Input, Restartable},
    Songbird,
};

pub async fn queue_search(
    manager: Arc<Songbird>,
    guild_id: GuildId,
    query: String,
) -> Result<(), &'static str> {
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
    manager: Arc<Songbird>,
    guild_id: GuildId,
    url: String,
) -> Result<(), &'static str> {
    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let restartable = match Restartable::ytdl(url, false).await {
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
