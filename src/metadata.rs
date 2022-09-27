use regex::Regex;
use serde::{Deserialize, Serialize};
use std::process;

#[derive(Serialize, Deserialize)]
struct YoutubeDLJson {
    _type: Option<String>,
    ie_key: Option<String>,
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<f32>,
}

pub fn get_playlist_sources(url: String) -> Result<Vec<String>, &'static str> {
    match process::Command::new("youtube-dl")
        .arg("--flat-playlist")
        .arg("-j")
        .arg(&url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl"),
        Ok(output) => {
            let mut sources: Vec<String> = Vec::new();

            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                println!("[playlist] [youtube-dl] {}", err_str);
            }

            let lines = output_str.split("\n");

            for line in lines {
                if line.is_empty() {
                    continue;
                }

                let json: YoutubeDLJson = serde_json::from_str(line).unwrap();

                if let Some(id) = json.id {
                    sources.push(id);
                }
            }

            return Ok(sources);
        }
    }
}

#[derive(Debug)]
pub struct Timestamp {
    seconds: i32,
    label: String,
    timestamp: String,
}

pub fn get_timestamps(description: String) -> Vec<Timestamp> {
    let mut timestamps: Vec<Timestamp> = Vec::new();

    let reg = Regex::new("[0-9]*:?[0-9]?[0-9]:[0-9][0-9]").unwrap();

    for line in description.split("\n") {
        if let Some(reg_match) = reg.find(line) {
            let timestamp_string = reg_match.as_str();

            let seconds = timestamp_string.split(":").fold(0, |accum, x| {
                accum * 60 + x.parse::<i32>().unwrap_or_default()
            });

            println!("{} -> {}", description, seconds);

            timestamps.push(Timestamp {
                seconds,
                label: line[reg_match.end()..line.len()].trim().to_string(),
                timestamp: timestamp_string.to_string(),
            })
        }
    }

    timestamps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playlist_success_page() {
        let sources = get_playlist_sources(
            "https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS".to_string(),
        )
        .unwrap();

        for source in sources.iter() {
            assert!(!source.is_empty())
        }
    }

    #[test]
    fn playlist_success_video() {
        let sources = get_playlist_sources(
            "https://www.youtube.com/watch?v=nBpgoga0FZ4&list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS"
                .to_string(),
        )
        .unwrap();

        for source in sources.iter() {
            assert!(!source.is_empty())
        }
    }

    #[test]
    fn playlist_fail_video_url() {
        let sources =
            get_playlist_sources("https://www.youtube.com/watch?v=6YBDo5S8soo".to_string())
                .unwrap();

        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn playlist_fail_not_url() {
        let sources = get_playlist_sources("amogus".to_string()).unwrap();

        assert!(sources.is_empty());
    }

    #[test]
    fn timestamps_single_line() {
        let timestamps = get_timestamps("3:23 My description".to_string());

        assert!(timestamps[0].label.eq("My description"));
        assert!(timestamps[0].timestamp.eq("3:23"));
        assert_eq!(timestamps[0].seconds, 3 * 60 + 23);
    }
}
