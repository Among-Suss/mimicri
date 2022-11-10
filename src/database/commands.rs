use crate::{
    utils::{
        config,
        responses::{self, Responses},
        strings,
    },
    CommandResult, Context,
};

use super::plugin::get_db_plugin;

pub async fn history(ctx: Context<'_>, page: usize) -> CommandResult {
    let guild_id = ctx.guild().unwrap().id;

    let page_size = config::queue::page_size(guild_id);
    let queue_text_len = config::queue::text_length(guild_id);

    if let Some(db_plugin) = get_db_plugin(ctx.discord()).await {
        if let Ok((history, count)) =
            db_plugin.get_history(ctx.author().id, page_size, page * page_size)
        {
            let mut description = String::new();

            for (i, info) in history.iter().enumerate() {
                description += &format!(
                    "{}. **{}**  [↗️]({})\n",
                    i + 1,
                    strings::escape_string(&strings::limit_string_length(
                        &info.title,
                        queue_text_len
                    )),
                    info.url
                )
                .to_string();
            }
            ctx.send(|m| {
                m.content("").embed(|e| {
                    responses::format_embed_playlist(e, history.iter(), count, guild_id, page)
                        .title(format!("{}'s History", ctx.author().name))
                        .color(config::colors::history())
                })
            })
            .await
            .expect("Failed to send message");
        } else {
            ctx.error("Database error, unable to fetch history.").await;
        }
    }
    Ok(())
}

pub async fn get_history(ctx: Context<'_>, page: usize) -> Option<String> {
    if let Some(db_plugin) = get_db_plugin(ctx.discord()).await {
        if let Ok((history, count)) = db_plugin.get_history(ctx.author().id, 1, page) {
            if history.len() > 0 {
                return Some(history[0].url.clone());
            } else {
                ctx.warn(format!(
                    "Song index not found. History contains {} songs.",
                    count
                ))
                .await;
            }
        } else {
            ctx.error("Unable to load history").await;
        }
    }

    None
}
