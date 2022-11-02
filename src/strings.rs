use std::cmp;

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
    let length = config::progress_bar_length(guild_id) as usize;

    let marker = config::progress_bar_marker(guild_id);
    let track = config::progress_bar_track(guild_id);

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
}
