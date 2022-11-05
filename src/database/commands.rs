use std::cmp;

use serenity::{
    framework::standard::{Args, CommandResult},
    model::prelude::Message,
    prelude::Context,
};

use crate::utils::{config, message_context::MessageContext, strings};

use super::plugin::get_db_plugin;

pub async fn history(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let message_ctx = MessageContext::new(ctx, msg);

    let page = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let db_plugin = get_db_plugin(ctx).await.unwrap().clone();

    let guild_id = msg.guild(&ctx.cache).unwrap().id;

    let page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    if let Ok((history, count)) =
        db_plugin.get_history(*msg.author.id.as_u64(), page_size, page * page_size)
    {
        let mut description = String::new();

        for (i, info) in history.iter().enumerate() {
            description += &format!(
                "{}. **{}**  [↗️]({})\n",
                i + 1,
                strings::escape_string(&strings::limit_string_length(&info.title, queue_text_len)),
                info.url
            )
            .to_string();
        }
        message_ctx
            .send_message(|m| {
                m.content("").embed(|e| {
                    MessageContext::format_embed_playlist(e, history.iter(), count, guild_id, page)
                        .title(format!("{}'s History", msg.author.name))
                        .color(config::colors::history())
                });

                m
            })
            .await;
    } else {
        message_ctx
            .send_error("Database error, unable to fetch history.")
            .await;
    }

    Ok(())
}

pub async fn play_history(ctx: &Context, msg: &Message, mut args: Args) -> Option<String> {
    let message_ctx = MessageContext::new(ctx, msg);

    let no = cmp::max(args.single::<i64>().unwrap_or_default() - 1, 0) as usize;

    let db_plugin = get_db_plugin(ctx).await.unwrap().clone();

    if let Ok((history, count)) = db_plugin.get_history(*msg.author.id.as_u64(), 1, no) {
        if history.len() > 0 {
            return Some(history[0].url.clone());
        } else {
            message_ctx
                .reply_warn(
                    msg,
                    format!("Song index not found. History contains {} songs.", count),
                )
                .await;
        }
    } else {
        message_ctx.reply_error(msg, "Unable to load history").await;
    }

    None
}
