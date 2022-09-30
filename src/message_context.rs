use std::{fmt::Display, sync::Arc};

use serenity::{
    builder::{CreateMessage, ParseValue},
    http::Http,
    model::prelude::{ChannelId, Message},
    Result as SerenityResult,
};

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

    pub async fn send_embed(
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
        println!("Error sending message: {:?}", why);
    }
}
