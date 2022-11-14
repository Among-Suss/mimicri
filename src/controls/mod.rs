pub mod commands;

use crate::{
    media::global_media_player::GlobalMediaPlayer, utils::responses::Responses, CommandResult,
    Context,
};

pub async fn join_channel(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    let guild = ctx.guild().unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states
        .get(&ctx.author().id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            ctx.info("Not in a voice channel").await;

            return Ok(());
        }
    };

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler = manager.join(guild_id, connect_to).await;

    if let Ok(_) = handler.1 {
        let res = media_player.start(guild_id, handler.0).await;
        match res {
            Ok(_) => (),
            Err(err) => {
                ctx.warn(err).await;
            }
        }
    }

    Ok(())
}
