use poise::command;

use super::*;

#[command(slash_command, prefix_command, category = "controls")]
pub async fn mute(ctx: Context<'_>) -> CommandResult {
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

#[command(slash_command, prefix_command, category = "controls")]
pub async fn deafen(ctx: Context<'_>) -> CommandResult {
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

#[command(slash_command, prefix_command, category = "controls")]
pub async fn unmute(ctx: Context<'_>) -> CommandResult {
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

#[command(slash_command, prefix_command, category = "controls")]
pub async fn undeafen(ctx: Context<'_>) -> CommandResult {
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
