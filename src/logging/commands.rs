use std::path::Path;

use poise::command;
use serenity::model::prelude::AttachmentType;

use crate::{utils::responses::Responses, CommandResult, Context};

#[command(slash_command, prefix_command, category = "debug")]
pub async fn log(ctx: Context<'_>, #[rest] args: String) -> CommandResult {
    let mut level = "".to_string();
    let mut target = "".to_string();
    let mut from: usize = 0;

    let mut level_flag = false;
    let mut target_flag = false;
    let mut from_flag = false;

    for arg in args.split(" ") {
        if arg == "-h" || arg == "--help" {
            ctx.info(super::format_help_message()).await;

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
        ctx.error(log_msgs.1).await;
    }

    ctx.send(|m| m.content(log_msgs.0))
        .await
        .expect("Failed to send message");

    Ok(())
}

#[command(slash_command, prefix_command, rename = "log-file", category = "debug")]
pub async fn log_file(ctx: Context<'_>) -> CommandResult {
    let log_file = super::get_log_filename();

    ctx.send(|m| m.attachment(AttachmentType::Path(Path::new(&log_file))))
        .await
        .expect("Failed to send message");

    Ok(())
}
