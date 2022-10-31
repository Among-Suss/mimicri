use std::env;

use ansi_term::Color;
use async_std::fs;
use serde::{Deserialize, Serialize};
use tracing::error;

pub fn get_log_filename() -> String {
    env::var("LOG_FILE").unwrap_or("output.txt".to_owned())
}

/// Returns (logs, warning)
pub async fn get_logs(level: String, target: String, start: usize) -> (String, String) {
    if let Ok(logs) = get_log().await {
        let mut level_flag = false;

        let level_filtered_logs: Vec<Log> = match level.to_uppercase().as_str() {
            "ERROR" => logs.into_iter().filter(|l| l.level == "ERROR").collect(),
            "WARN" => logs
                .into_iter()
                .filter(|l| l.level == "ERROR" || l.level == "WARN")
                .collect(),
            "INFO" => logs,
            "" => logs,
            _ => {
                level_flag = true;

                logs
            }
        };

        let target_filtered_logs: Vec<Log> = level_filtered_logs
            .into_iter()
            .filter(|l| l.target.contains(&target))
            .collect();

        let log_msgs = target_filtered_logs
            .into_iter()
            .map(format_log)
            .collect::<Vec<String>>();

        let mut char_count = 0;
        let mut line_count = 0;

        while line_count < log_msgs.len() && char_count < 1950 {
            char_count += log_msgs[line_count].len();

            line_count += 1;
        }

        let mut warn_msg = "".to_string();

        if level_flag {
            warn_msg += &format!(
                "[WARNING] Unknown level '{}'. Available targets: ERROR, WARN, INFO.\n",
                level
            )
        };

        let to = log_msgs.len();
        let from = to - line_count + 1;

        let log_msg = log_msgs
            .into_iter()
            .skip(start + from)
            .take(to)
            .collect::<Vec<String>>()
            .join("\n");

        (
            if !log_msg.is_empty() {
                format!("```ansi\n{}\n```", log_msg)
            } else {
                "`No logs found`".to_string()
            },
            warn_msg,
        )
    } else {
        ("".to_string(), "Unable to get log file".to_string())
    }
}

pub fn format_help_message() -> String {
    format!(
        "```ansi\nusage: {} {}\n```",
        Color::White.bold().paint(format!(
            "{}log",
            env::var("BOT_PREFIX").unwrap_or("~".to_owned())
        )),
        "[--level <ERROR | WARN | INFO>] [--target <mimicri | serenity | ...>] [--from (int)]"
    )
}

async fn get_log() -> Result<Vec<Log>, String> {
    if let Ok(file_data) = fs::read_to_string(get_log_filename()).await {
        Ok(file_data
            .split("\n")
            .filter(|s| !s.is_empty())
            .filter_map(|s| match serde_json::from_str::<Log>(s) {
                Ok(json) => Some(json),
                Err(err) => {
                    error!("{}", err);
                    None
                }
            })
            .collect())
    } else {
        Err("Unable to get file".to_owned())
    }
}

fn format_log(log: Log) -> String {
    let timestamp_str = log.timestamp;
    let level_str = match log.level.as_str() {
        "ERROR" => Color::Red.bold().paint(log.level).to_string(),
        "WARN" => Color::Yellow.bold().paint(log.level).to_string(),
        "INFO" => Color::Cyan.bold().paint(log.level).to_string(),
        _ => log.level,
    };
    let message = Color::White.paint(log.fields.message);

    format!("{} {} {}", timestamp_str, level_str, message)
}

#[derive(Serialize, Deserialize)]
struct Log {
    timestamp: String,
    level: String,
    target: String,
    fields: LogJsonFields,
}

#[derive(Serialize, Deserialize)]
struct LogJsonFields {
    message: String,
}
