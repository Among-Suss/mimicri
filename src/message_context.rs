use std::{fmt::Display, sync::Arc};

use serenity::{
    builder::{CreateEmbed, CreateMessage, ParseValue},
    http::Http,
    model::prelude::{ChannelId, Message},
    prelude::Context,
    Result as SerenityResult,
};
use tracing::error;

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
                .send_message(self.http.clone(), |m| {
                    m.content(message);
                    m
                })
                .await,
        );
    }

    pub async fn send_error(&self, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content(message);
                    m
                })
                .await,
        );
    }

    pub async fn send_simple_embed(
        &self,
        message: impl Display,
        title: impl Display,
        description: impl Display,
    ) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content(message)
                        .embed(|e| e.title(title).description(description));
                    m
                })
                .await,
        );
    }

    pub async fn reply(&self, reply_message: &Message, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content(message)
                        .reference_message(reply_message)
                        .allowed_mentions(|f| {
                            f.replied_user(false)
                                .parse(ParseValue::Everyone)
                                .parse(ParseValue::Users)
                                .parse(ParseValue::Roles)
                        });
                    m
                })
                .await,
        );
    }

    pub async fn reply_error(&self, reply_message: &Message, message: impl Display) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content(message)
                        .reference_message(reply_message)
                        .allowed_mentions(|f| {
                            f.replied_user(false)
                                .parse(ParseValue::Everyone)
                                .parse(ParseValue::Users)
                                .parse(ParseValue::Roles)
                        });
                    m
                })
                .await,
        );
    }

    pub async fn reply_embed<F>(&self, reply_message: &Message, embed_callback: F)
    where
        F: FnOnce(&mut CreateEmbed) -> &mut CreateEmbed,
    {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content("")
                        .embed(embed_callback)
                        .reference_message(reply_message)
                        .allowed_mentions(|f| {
                            f.replied_user(false)
                                .parse(ParseValue::Everyone)
                                .parse(ParseValue::Users)
                                .parse(ParseValue::Roles)
                        });
                    m
                })
                .await,
        );
    }

    pub async fn reply_basic_embed(
        &self,
        reply_message: &Message,
        message: impl Display,
        title: impl Display,
        description: impl Display,
    ) {
        check_msg(
            self.channel
                .send_message(self.http.clone(), |m| {
                    m.content(message)
                        .embed(|e| e.title(title).description(description))
                        .reference_message(reply_message)
                        .allowed_mentions(|f| {
                            f.replied_user(false)
                                .parse(ParseValue::Everyone)
                                .parse(ParseValue::Users)
                                .parse(ParseValue::Roles)
                        });
                    m
                })
                .await,
        );
    }
}

pub fn create_info_message(
    m: &mut CreateMessage,
    message: &String,
    title: &String,
    description: &String,
) {
    m.content(message)
        .embed(|e| e.title(title).description(description));
}

pub fn create_replay_message(
    m: &mut CreateMessage,
    msg: &Message,
    message: &String,
    title: &String,
    description: &String,
) {
    create_info_message(m, message, title, description);
    m.content(message)
        .reference_message(msg)
        .allowed_mentions(|f| {
            f.replied_user(false)
                .parse(ParseValue::Everyone)
                .parse(ParseValue::Users)
                .parse(ParseValue::Roles)
        });
}

fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        error!("Error sending message: {:?}", why);
    }
}
