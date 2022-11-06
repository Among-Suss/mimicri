use std::{fmt::Display, sync::Arc};

use serenity::{
    builder::{CreateEmbed, CreateInteractionResponseData, CreateMessage, ParseValue},
    http::Http,
    model::prelude::{ChannelId, GuildId, Message},
    prelude::Context,
    Result as SerenityResult,
};
use tracing::error;

use crate::{config, media::media_info::MediaInfo};

use super::strings;

pub struct MessageContext {
    pub channel: ChannelId,
    pub http: Arc<Http>,
}

impl Clone for MessageContext {
    fn clone(&self) -> Self {
        Self {
            channel: self.channel.clone(),
            http: self.http.clone(),
        }
    }
}

impl MessageContext {
    pub fn new(ctx: &Context, msg: &Message) -> MessageContext {
        MessageContext {
            channel: msg.channel_id,
            http: ctx.http.clone(),
        }
    }

    pub async fn send_message<'a, F>(&self, callback: F)
    where
        for<'b> F: FnOnce(&'b mut CreateMessage<'a>) -> &'b mut CreateMessage<'a>,
    {
        Self::check_msg(self.channel.send_message(self.http.clone(), callback).await);
    }

    pub async fn send_info(&self, message: impl Display) {
        Self::check_msg(
            self.channel
                .send_message(self.http.clone(), |m| Self::format_info(m, message))
                .await,
        );
    }

    pub async fn send_error(&self, message: impl Display) {
        Self::check_msg(
            self.channel
                .send_message(self.http.clone(), |m| Self::format_error(m, message))
                .await,
        );
    }

    pub async fn reply_info(&self, reference: &Message, message: impl Display) {
        Self::check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    Self::format_reply(Self::format_info(m, message), reference, false)
                })
                .await,
        );
    }
    pub async fn reply_error(&self, reference: &Message, message: impl Display) {
        Self::check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    Self::format_reply(Self::format_error(m, message), reference, true)
                })
                .await,
        );
    }

    pub async fn reply_warn(&self, reference: &Message, message: impl Display) {
        Self::check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    Self::format_reply(Self::format_warn(m, message), reference, true)
                })
                .await,
        );
    }

    pub fn format_info<'a, 'b>(
        m: &'b mut CreateMessage<'a>,
        message: impl Display,
    ) -> &'b mut CreateMessage<'a> {
        m.content("").embed(|e| {
            e.title("Info")
                .description(&message)
                .color(config::colors::info())
        })
    }

    pub fn format_interaction_info<'a, 'b>(
        m: &'b mut CreateInteractionResponseData<'a>,
        message: impl Display,
    ) -> &'b mut CreateInteractionResponseData<'a> {
        m.content("").embed(|e| {
            e.title("Info")
                .description(&message)
                .color(config::colors::info())
        })
    }

    pub fn format_error<'a, 'b>(
        m: &'b mut CreateMessage<'a>,
        message: impl Display,
    ) -> &'b mut CreateMessage<'a> {
        m.content("").embed(|e| {
            e.title("Error")
                .description(&message)
                .color(config::colors::error())
        })
    }

    pub fn format_warn<'a, 'b>(
        m: &'b mut CreateMessage<'a>,
        message: impl Display,
    ) -> &'b mut CreateMessage<'a> {
        m.content("").embed(|e| {
            e.title("Warning")
                .description(&message)
                .color(config::colors::warn())
        })
    }

    pub fn format_reply<'a, 'b>(
        m: &'b mut CreateMessage<'a>,
        reference: &Message,
        do_mention: bool,
    ) -> &'b mut CreateMessage<'a> {
        m.reference_message(reference).allowed_mentions(|f| {
            if !do_mention {
                f.replied_user(false)
                    .parse(ParseValue::Everyone)
                    .parse(ParseValue::Users)
                    .parse(ParseValue::Roles)
            } else {
                f
            }
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
                        strings::escape_string(&strings::limit_string_length(
                            &info.title,
                            text_len,
                        )),
                        info.url,
                        strings::format_timestamp(info.duration)
                    )
                })
                .collect::<Vec<String>>()
                .join("\n"),
        )
        .footer(|f| {
            f.text(format!(
                "Page {} of {} ({} track(s))",
                page + 1,
                total / page_size + 1,
                total
            ))
        });

        e
    }

    fn check_msg(result: SerenityResult<Message>) {
        if let Err(why) = result {
            error!("Error sending message: {:?}", why);
        }
    }
}
