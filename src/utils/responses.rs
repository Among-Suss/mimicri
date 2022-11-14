use std::fmt::Display;

use poise::{
    async_trait,
    serenity_prelude::{CreateEmbed, GuildId},
    CreateReply, ReplyHandle,
};
use tracing::error;

use crate::{media::media_info::MediaInfo, Context};

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
