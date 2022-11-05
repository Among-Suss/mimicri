use serenity::model::prelude::GuildId;

use crate::{
    media::{media::GlobalMediaPlayer, media_info::MediaInfo},
    message_context::MessageContext,
    metadata::{get_info, get_playlist, get_search, is_playlist},
};

pub async fn queue_variant(
    guild_id: GuildId,
    query: &String,
    message_ctx: MessageContext,
    global_media_player: &GlobalMediaPlayer,
) -> Result<MediaInfo, String> {
    if is_playlist(query) {
        let infos = match get_playlist(query) {
            Ok(infos) => infos,
            Err(err) => return Err(err),
        };

        if infos.len() == 0 {
            return Err("Playlist is empty!".to_string());
        }

        // FIXME Is cloning a linked list okay?
        global_media_player
            .enqueue_batch(guild_id, infos.clone(), message_ctx)
            .await?;

        Ok(infos.into_iter().nth(0).unwrap())
    } else {
        let info = if query.starts_with("http") {
            match get_info(query) {
                Ok(url) => url,
                Err(err) => return Err(err),
            }
        } else {
            match get_search(query) {
                Ok(url) => url,
                Err(err) => return Err(err),
            }
        };

        global_media_player
            .enqueue(guild_id, info.clone(), message_ctx)
            .await?;

        Ok(info)
    }
}
