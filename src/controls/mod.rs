pub mod commands;

use crate::{
    media::global_media_player::GlobalMediaPlayer, utils::responses::Responses, CommandResult,
    Context,
};

pub async fn join(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
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

pub async fn leave(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        let res = media_player.quit(guild_id).await;
        match res {
            Ok(_) => (),
            Err(err) => ctx.warn(err).await,
        }

        if let Err(err) = manager.remove(guild_id).await {
            ctx.error(format!("Failed: {:?}", err)).await;
        }

        ctx.info("Left voice channel").await;
    } else {
        ctx.info("Not in a voice channel").await;
    }

    Ok(())
}
