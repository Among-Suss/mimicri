use std::{fmt::Display, future::Future};

use poise::{
    async_trait,
    serenity_prelude::{
        ButtonStyle, CollectComponentInteraction, CreateComponents, CreateEmbed, GuildId,
        InteractionResponseType,
    },
    CreateReply, ReplyHandle,
};
use tracing::error;

use crate::{media::media_info::MediaInfo, CommandResult, Context};

use super::{config, strings};

#[async_trait]
pub trait Responses {
    async fn info(self, message: impl Display + std::marker::Send);
    async fn warn(self, message: impl Display + std::marker::Send);
    async fn error(self, message: impl Display + std::marker::Send);
}

#[async_trait]
impl Responses for Context<'_> {
    async fn info(self, message: impl Display + std::marker::Send) {
        check_msg(self.send(|m| format_info(m, message)).await);
    }

    async fn warn(self, message: impl Display + std::marker::Send) {
        check_msg(self.send(|m| format_warn(m, message)).await);
    }

    async fn error(self, message: impl Display + std::marker::Send) {
        check_msg(self.send(|m| format_error(m, message)).await);
    }
}

fn check_msg(response_result: Result<ReplyHandle<'_>, serenity::Error>) {
    if let Err(err) = response_result {
        error!("Failed to send message: {}", err);
    }
}

pub fn format_info<'a, 'b>(
    m: &'b mut CreateReply<'a>,
    message: impl Display,
) -> &'b mut CreateReply<'a> {
    m.content("").embed(|e| {
        e.title("Info")
            .description(&message)
            .color(config::colors::info())
    })
}

pub fn format_error<'a, 'b>(
    m: &'b mut CreateReply<'a>,
    message: impl Display,
) -> &'b mut CreateReply<'a> {
    m.content("").embed(|e| {
        e.title("Error")
            .description(&message)
            .color(config::colors::error())
    })
}

pub fn format_warn<'a, 'b>(
    m: &'b mut CreateReply<'a>,
    message: impl Display,
) -> &'b mut CreateReply<'a> {
    m.content("").embed(|e| {
        e.title("Warning")
            .description(&message)
            .color(config::colors::warn())
    })
}

pub fn format_embed_playlist<'a, 'b, I>(
    e: &'b mut CreateEmbed,
    songs: I,
    total: usize,
    guild_id: GuildId,
    page: usize,
) -> &'b mut CreateEmbed
where
    I: Iterator<Item = &'a MediaInfo>,
{
    let page_size = config::queue::page_size(guild_id);
    let text_len = config::queue::text_length(guild_id);

    e.description(
        songs
            .enumerate()
            .map(|(i, info)| {
                format!(
                    "**{}) [{}]({})** ({})",
                    i + 1 + page * page_size,
                    strings::escape_string(&strings::limit_string_length(&info.title, text_len,)),
                    info.url,
                    strings::format_timestamp(info.duration)
                )
            })
            .collect::<Vec<String>>()
            .join("\n"),
    )
    .footer(|f| f.text(strings::page_display(page + 1, total, page_size, "track")));

    e
}

fn create_buttons(
    c: &mut CreateComponents,
    id: u64,
    page: usize,
    total: usize,
) -> &mut CreateComponents {
    let back = page > 0;
    let next = page < total - 1;

    if next || back {
        c.create_action_row(|r| {
            if back {
                r.create_button(|b| {
                    b.label("Back")
                        .custom_id(format!("{}__back", id))
                        .style(ButtonStyle::Primary)
                });
            }
            if next {
                r.create_button(|b| {
                    b.label("Next")
                        .custom_id(format!("{}__next", id))
                        .style(ButtonStyle::Primary)
                });
            }
            r
        });
    }

    c
}

pub async fn create_pagination<F, Fut>(
    ctx: Context<'_>,
    initial_page: usize,
    update: F,
) -> CommandResult
where
    F: Fn(usize) -> Fut,
    Fut: Future<Output = Result<(CreateEmbed, usize, usize), String>>,
{
    let id = ctx.id();

    let (embed, page, total_page) = match update(initial_page).await {
        Ok(res) => res,
        Err(err) => {
            ctx.error(err).await;
            return Ok(());
        }
    };

    ctx.send(|m| {
        m.embeds.push(embed);
        m.components(|c| create_buttons(c, id, page, total_page))
    })
    .await?;

    let mut page = page;

    while let Some(mci) = CollectComponentInteraction::new(ctx.discord())
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .timeout(std::time::Duration::from_secs(120))
        .filter(move |mci| mci.data.custom_id.contains(id.to_string().as_str()))
        .await
    {
        let delta: i64 = if mci.data.custom_id.contains("__back") {
            -1
        } else if mci.data.custom_id.contains("__next") {
            1
        } else {
            0
        };

        let next = (page as i64 + delta) as usize;

        match update(next).await {
            Ok((embed, new_page, total_page)) => {
                page = new_page;
                let mut msg = mci.message.clone();
                msg.edit(ctx.discord(), |m| {
                    m.set_embed(embed)
                        .components(|c| create_buttons(c, id, new_page, total_page))
                })
                .await?;

                mci.create_interaction_response(ctx.discord(), |ir| {
                    ir.kind(InteractionResponseType::DeferredUpdateMessage)
                })
                .await?;
            }
            Err(msg) => ctx.error(msg).await,
        }
    }

    Ok(())
}
