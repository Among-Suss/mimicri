use tracing::info;

use crate::{
    media::global_media_player::GlobalMediaPlayer, utils::responses::Responses, CommandResult,
    Context,
};

pub async fn join(media_player: &GlobalMediaPlayer, ctx: Context<'_>) -> CommandResult {
    info!("Joining...");

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

pub async fn leave(media_player: &GlobalMediaPlayer, ctx: &Context<'_>) -> CommandResult {
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

pub async fn mute(ctx: &Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.warn("Not in a voice channel").await;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        ctx.warn("Already muted").await
    } else {
        if let Err(e) = handler.mute(true).await {
            ctx.error(format!("Failed: {:?}", e)).await;
        }

        ctx.info("Now muted").await;
    }

    Ok(())
}

pub async fn deafen(ctx: &Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.warn("Not in a voice channel").await;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        ctx.warn("Already deafened").await;
    } else {
        if let Err(e) = handler.deafen(true).await {
            ctx.error(format!("Failed: {:?}", e)).await;
        }

        ctx.info("Deafened").await;
    }

    Ok(())
}

pub async fn unmute(ctx: &Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            ctx.error(format!("Failed: {:?}", e)).await;
        }

        ctx.info("Unmuted").await;
    } else {
        ctx.warn("Not in a voice channel to unmute in").await;
    }

    Ok(())
}

pub async fn undeafen(ctx: &Context<'_>) -> CommandResult {
    let guild_id = ctx.guild_id().unwrap();

    let manager = songbird::get(ctx.discord())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            ctx.error(format!("Failed: {:?}", e)).await;
        }

        ctx.info("Undeafened").await;
    } else {
        ctx.warn("Not in a voice channel to undeafen in").await;
    }

    Ok(())
}
