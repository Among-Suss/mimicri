use std::{fmt::Display, sync::Arc};

use serenity::{
    builder::{CreateMessage, ParseValue},
    http::Http,
    model::prelude::{ChannelId, Message},
    prelude::Context,
    Result as SerenityResult,
};
use tracing::error;

use crate::config;

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
        check_msg(self.channel.send_message(self.http.clone(), callback).await);
    }

    pub async fn send_info(&self, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| format_info(m, message))
                .await,
        );
    }

    pub async fn send_error(&self, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| format_error(m, message))
                .await,
        );
    }

    pub async fn reply_info(&self, reference: &Message, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    format_reply(format_info(m, message), reference, false)
                })
                .await,
        );
    }
    pub async fn reply_error(&self, reference: &Message, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    format_reply(format_error(m, message), reference, true)
                })
                .await,
        );
    }
}

fn format_info<'a, 'b>(
    m: &'b mut CreateMessage<'a>,
    message: impl Display,
) -> &'b mut CreateMessage<'a> {
    m.content("").embed(|e| {
        e.title("Info")
            .description(&message)
            .color(config::colors::info())
    })
}

fn format_error<'a, 'b>(
    m: &'b mut CreateMessage<'a>,
    message: impl Display,
) -> &'b mut CreateMessage<'a> {
    m.content("").embed(|e| {
        e.title("Error")
            .description(&message)
            .color(config::colors::error())
    })
}

fn format_reply<'a, 'b>(
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

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
