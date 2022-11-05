use std::path::Path;

use serenity::{
    framework::standard::{Args, CommandResult},
    model::prelude::{AttachmentType, Message},
    prelude::Context,
};

use crate::utils::message_context::MessageContext;

pub async fn log(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let msg_ctx = MessageContext {
        channel: msg.channel_id,
        http: ctx.http.clone(),
    };

    let mut level = "".to_string();
    let mut target = "".to_string();
    let mut from: usize = 0;

    let mut level_flag = false;
    let mut target_flag = false;
    let mut from_flag = false;

    for arg in args.raw() {
        if arg == "-h" || arg == "--help" {
            msg_ctx.send_info(super::format_help_message()).await;

            return Ok(());
        } else if arg == "-l" || arg == "--level" {
            level_flag = true;
        } else if arg == "-t" || arg == "--target" {
            target_flag = true;
        } else if arg == "-f" || arg == "--from" {
            from_flag = true;
        } else if level_flag {
            level = arg.to_string();
            level_flag = false;
        } else if target_flag {
            target = arg.to_string();
            target_flag = false;
        } else if from_flag {
            from = arg.parse::<usize>().unwrap_or_default();
            from_flag = false;
        }
    }

    let log_msgs = super::get_logs(level, target, from).await;

    if !log_msgs.1.is_empty() {
        msg_ctx.send_error(log_msgs.1).await;
    }

    msg_ctx
        .send_message(|m| MessageContext::format_reply(m.content(log_msgs.0), msg, false))
        .await;

    Ok(())
}

pub async fn log_file(ctx: &Context, msg: &Message) -> CommandResult {
    let log_file = super::get_log_filename();

    let _ = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.files(vec![AttachmentType::Path(Path::new(&log_file))])
        })
        .await;

    Ok(())
}
