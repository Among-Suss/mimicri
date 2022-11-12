use std::cmp;

use regex::Regex;
use serenity::model::prelude::GuildId;

use crate::config;

static SENSITIVE_CHARACTERS: [&str; 7] = ["\\", "*", "_", "~", "`", "|", ">"];

/// Escapes all sensitize Discord characters
/* @see: https://github.com/discord-net/Discord.Net/blob/265da99619a775d23b24326648fe4220bc6beeae/src/Discord.Net.Core/Format.cs#L36 */
pub fn escape_string(text: &String) -> String {
    let mut sanitized_text = text.clone();

    for i in 0..SENSITIVE_CHARACTERS.len() {
        sanitized_text = sanitized_text.replace(
            SENSITIVE_CHARACTERS[i],
            format!("\\{}", SENSITIVE_CHARACTERS[i]).as_str(),
        );
    }

    sanitized_text
}

/// Slices the string from zero to width, and rounds to the nearest code point
/// Does not account for unicode size, as unicode characters tend to be larger.
pub fn limit_string_length(text: &String, width: usize) -> String {
    if text.len() <= width {
        return text.clone();
    }

    let mut previous_code_point = 0;
    let mut previous_previous_code_point = 0;

    for code_point in text.char_indices().map(|(i, _)| i).into_iter() {
        if code_point > width {
            return text.clone()[0..previous_previous_code_point].to_string() + "…";
        }

        previous_previous_code_point = previous_code_point;
        previous_code_point = code_point;
    }

    return text.clone();
}

pub fn create_progress_bar(guild_id: GuildId, percent: f32) -> String {
    let length = config::progress_bar::length(guild_id) as usize;
    let marker = config::progress_bar::marker(guild_id);
    let track = config::progress_bar::track(guild_id);

    generate_marker_progress_bar(length, percent, &marker, &track)
}

fn generate_marker_progress_bar(
    length: usize,
    percent: f32,
    marker: &String,
    track: &String,
) -> String {
    let total_count = length as i32;
    let display_count = (length as f32 * percent) as i32;

    track.repeat(cmp::min(display_count, total_count - 1) as usize)
        + &marker
        + &track.repeat(cmp::max(total_count - display_count - 1, 0) as usize)
}

pub fn format_timestamp(duration: i64) -> String {
    let seconds = duration % 60;
    let minutes = duration / 60 % 60;
    let hours = duration / 3600;

    if hours <= 0 {
        format!("{}:{:02}", minutes, seconds).to_string()
    } else {
        format!("{}:{:02}:{:02}", hours, minutes, seconds).to_string()
    }
}

pub fn parse_timestamp(timestamp: &String) -> i64 {
    timestamp.split(":").fold(0, |accum, x| {
        accum * 60 + x.parse::<i64>().unwrap_or_default()
    })
}

pub fn is_url(text: &String) -> bool {
    text.starts_with("https://")
}

pub struct Timestamp {
    pub seconds: i64,
    pub label: String,
    pub timestamp: String,
    pub string: String,
}

pub fn parse_description_timestamps(description: String) -> Vec<Timestamp> {
    let mut timestamps: Vec<Timestamp> = Vec::new();

    let reg = Regex::new("[0-9]*:?[0-9]?[0-9]:[0-9][0-9]").unwrap();

    for line in description.split("\n") {
        if let Some(reg_match) = reg.find(line) {
            let timestamp_string = reg_match.as_str();

            let seconds = parse_timestamp(&timestamp_string.to_string());

            let front = line[0..reg_match.start()].trim();
            let back = line[reg_match.end()..line.len()].trim();

            timestamps.push(Timestamp {
                string: line.to_string(),
                seconds,
                label: format!(
                    "{}{}{}",
                    front,
                    if front.len() > 0 && back.len() > 0 {
                        " "
                    } else {
                        ""
                    },
                    back
                )
                .to_string(),
                timestamp: timestamp_string.to_string(),
            })
        }
    }

    timestamps
}

pub fn is_timestamp(string: &String) -> bool {
    let reg = Regex::new("[0-9]*:?[0-9]?[0-9]:[0-9][0-9]").unwrap();

    reg.is_match(string)
}

#[cfg(test)]
mod tests {
    use super::*;

    mod progress_bar {
        use super::generate_marker_progress_bar;

        #[test]
        fn start() {
            let bar = generate_marker_progress_bar(10, 0.0, &"x".to_string(), &"-".to_string());

            assert_eq!(bar, "x---------")
        }

        #[test]
        fn end() {
            let bar = generate_marker_progress_bar(10, 1.0, &"x".to_string(), &"-".to_string());

            assert_eq!(bar, "---------x")
        }
    }

    mod escape_string {
        use super::escape_string;

        #[test]
        fn escaping_string() {
            let bad_string = "I \\ have * a _ very ~ bad ` string | here >".to_string();
            let escaped_string = escape_string(&bad_string);

            assert_eq!(
                escaped_string,
                "I \\\\ have \\* a \\_ very \\~ bad \\` string \\| here \\>"
            );
        }
    }

    mod slice_string {
        use super::limit_string_length;

        #[test]
        fn string_exact() {
            let long_string = "I want to die now".to_string();
            let cut_string = limit_string_length(&long_string, 17);

            assert_eq!(cut_string, "I want to die now");
        }

        #[test]
        fn string_longer() {
            let long_string = "I want to die now".to_string();
            let cut_string = limit_string_length(&long_string, 14);

            assert_eq!(cut_string, "I want to die…");
        }

        #[test]
        fn string_shorter() {
            let long_string = "I want to die".to_string();
            let cut_string = limit_string_length(&long_string, 17);

            assert_eq!(cut_string, "I want to die");
        }

        #[test]
        /// As long as it doesn't panic it's fine
        fn string_unicode() {
            let long_string = "人生は意味がない".to_string();
            let cut_string = limit_string_length(&long_string, 7);

            assert_eq!(cut_string, "人…");
        }

        #[test]
        fn string_unicode_2() {
            let long_string = "人生は意味がない".to_string();
            let cut_string = limit_string_length(&long_string, 10);

            assert_eq!(cut_string, "人生…");
        }

        #[test]
        fn string_unicode_longer() {
            let long_string = "人生は意味がない".to_string();
            let cut_string = limit_string_length(&long_string, 25);

            assert_eq!(cut_string, "人生は意味がない");
        }

        #[test]
        /// Not really expected behavior, but I don't think anyone is slicing string with 1 anyway
        fn string_one() {
            let long_string = "I want to die".to_string();
            let cut_string = limit_string_length(&long_string, 1);

            assert_eq!(cut_string, "…");
        }
    }

    mod format_timestamp {
        use super::format_timestamp;

        #[test]
        fn seconds() {
            assert_eq!(format_timestamp(30), "0:30")
        }

        #[test]
        fn minute_whole() {
            assert_eq!(format_timestamp(60), "1:00")
        }

        #[test]
        fn minute_seconds() {
            assert_eq!(format_timestamp(90), "1:30")
        }

        #[test]
        fn minute_seconds_near_minute() {
            assert_eq!(format_timestamp(60 + 59), "1:59")
        }

        #[test]
        fn minute_multiple() {
            assert_eq!(format_timestamp(60 + 60), "2:00")
        }

        #[test]
        fn minute_near_hour() {
            assert_eq!(format_timestamp(60 * 59 + 59), "59:59")
        }

        #[test]
        fn hour() {
            assert_eq!(format_timestamp(60 * 60), "1:00:00")
        }

        #[test]
        fn hour_seconds() {
            assert_eq!(format_timestamp(60 * 60 + 59), "1:00:59")
        }

        #[test]
        fn hour_minutes_seconds() {
            assert_eq!(format_timestamp(60 * 60 + 60 + 59), "1:01:59")
        }
    }

    mod parse_timestamp {
        use super::*;

        #[test]
        fn parses_seconds() {
            assert_eq!(parse_timestamp(&"0:30".to_string()), 30);
        }

        #[test]
        fn parses_minutes() {
            assert_eq!(parse_timestamp(&"1:30".to_string()), 90);
        }

        #[test]
        fn parses_hour() {
            assert_eq!(parse_timestamp(&"1:01:30".to_string()), 3690);
        }

        #[test]
        fn parses_bad_string() {
            assert_eq!(parse_timestamp(&"hello".to_string()), 0);
        }
    }

    mod description_timestamp {

        use super::parse_description_timestamps;

        #[test]
        fn single_line() {
            let timestamps = parse_description_timestamps("3:23 My description".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);
        }

        #[test]
        fn mid_line() {
            let timestamps = parse_description_timestamps(
                "Description in front 5:55 Description behind".to_string(),
            );

            assert_eq!(
                timestamps[0].label,
                "Description in front Description behind"
            );
            assert_eq!(timestamps[0].timestamp, "5:55");
            assert_eq!(timestamps[0].seconds, 5 * 60 + 55);
        }

        #[test]
        fn multi_line() {
            let timestamps =
                parse_description_timestamps("3:23 My description\n1:06:34 Other desc".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);

            assert_eq!(timestamps[1].label, "Other desc");
            assert_eq!(timestamps[1].timestamp, "1:06:34");
            assert_eq!(timestamps[1].seconds, 1 * 3600 + 6 * 60 + 34);
        }

        #[test]
        fn empty() {
            let timestamps = parse_description_timestamps("".to_string());

            assert_eq!(timestamps.len(), 0);
        }

        #[test]
        fn no_timestamps() {
            let timestamps =
                parse_description_timestamps("Hi there\n23 susser\nimposter".to_string());

            assert_eq!(timestamps.len(), 0);
        }
    }
}
