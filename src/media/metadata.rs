use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::LinkedList, process};
use tracing::error;

use crate::strings::parse_timestamp;

use super::media_info::{MediaInfo, PlaylistInfo};

#[derive(Serialize, Deserialize)]
struct YoutubeDLJson {
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    webpage_url: Option<String>,
    uploader: Option<String>,
    channel: Option<String>,
    // playlist
    playlist_title: Option<String>,
    playlist_uploader: Option<String>,
    playlist_index: Option<i64>,
}

#[derive(Serialize, Deserialize)]
struct YoutubeDLFlatJson {
    ie_key: Option<String>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<f64>,
    uploader: Option<String>,
}

impl From<YoutubeDLJson> for MediaInfo {
    fn from(json: YoutubeDLJson) -> Self {
        MediaInfo {
            url: json.webpage_url.unwrap_or_default(),
            title: json.title.unwrap_or_default(),
            description: json.description.unwrap_or_default(),
            duration: json.duration.unwrap_or_default() as i64,
            thumbnail: json.thumbnail.unwrap_or_default(),
            uploader: json.uploader.unwrap_or_default(),
            playlist: match json.playlist_title {
                Some(playlist_title) => Some(PlaylistInfo {
                    title: playlist_title,
                    uploader: json.playlist_uploader.unwrap_or_default(),
                }),
                None => None,
            },
        }
    }
}

impl From<YoutubeDLFlatJson> for MediaInfo {
    fn from(json: YoutubeDLFlatJson) -> Self {
        let platform = json.ie_key.unwrap_or_default();

        let url = if platform == "Youtube" {
            "https://www.youtube.com/watch?v=".to_string() + &json.id.unwrap_or_default()
        } else {
            "".to_string()
        };

        MediaInfo {
            url,
            title: json.title.unwrap_or_default(),
            duration: json.duration.unwrap_or_default() as i64,
            description: json.description.unwrap_or_default(),
            uploader: json.uploader.unwrap_or_default(),
            thumbnail: "".to_string(), // FIXME
            playlist: None,
        }
    }
}

pub fn get_info(url: &String) -> Result<MediaInfo, String> {
    match process::Command::new("youtube-dl")
        .arg("-j")
        .arg("--no-playlist")
        .arg(url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                error!("[metadata] [youtube-dl] {}", err_str);
            }

            let json_result: serde_json::Result<YoutubeDLJson> = serde_json::from_str(&output_str);

            match json_result {
                Err(err) => {
                    error!("[metadata] [youtube-dl] {}", err);
                    Err("Unable to parse json".to_string())
                }
                Ok(json) => {
                    if json.url.is_some() {
                        return Err("[metadata] [youtube-dl] Json returned no URL".to_string());
                    }

                    Ok(MediaInfo::from(json))
                }
            }
        }
    }
}

pub fn get_search(query: &String) -> Result<MediaInfo, String> {
    get_info(&format!("ytsearch:{}", query))
}

pub fn get_playlist(url: &String) -> Result<LinkedList<MediaInfo>, String> {
    let mut sources: LinkedList<MediaInfo> = LinkedList::new();

    match process::Command::new("youtube-dl")
        .arg("-j")
        .arg("--playlist-end=1")
        .arg(&url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                error!("[playlist] {}", err_str);
            }

            if let Some(line) = output_str.split("\n").nth(0) {
                let json_result: serde_json::Result<YoutubeDLJson> = serde_json::from_str(line);

                match json_result {
                    Ok(json) => sources.push_back(MediaInfo::from(json)),
                    Err(err) => error!("[playlist] {}", err),
                }
            } else {
                error!("[playlist] First song is empty");
            }
        }
    }

    match process::Command::new("youtube-dl")
        .arg("-j")
        .arg("--flat-playlist")
        .arg(&url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                error!("[playlist] {}", err_str);
            }

            let mut lines = output_str.split("\n");

            lines.next();

            for line in lines {
                if line.is_empty() {
                    continue;
                }

                let json_result: serde_json::Result<YoutubeDLFlatJson> = serde_json::from_str(line);

                match json_result {
                    Ok(json) => sources.push_back(MediaInfo::from(json)),
                    Err(err) => error!("[playlist] {}", err),
                }
            }

            return Ok(sources);
        }
    }
}

pub fn is_playlist(url: &String) -> bool {
    return url.contains("youtube.com")
        && (url.contains("/playlist?list=") || url.contains("&list="));
}

pub struct Timestamp {
    pub seconds: i64,
    pub label: String,
    pub timestamp: String,
    pub string: String,
}

pub fn parse_timestamps(description: String) -> Vec<Timestamp> {
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

#[cfg(test)]
mod tests {
    mod search {
        use crate::metadata::get_search;

        #[test]
        fn success() {
            let video = get_search(&"hello".to_string()).unwrap();

            assert!(!video.url.is_empty());
        }
    }

    mod playlist {
        use super::super::get_playlist;

        #[test]
        fn success_page() {
            let sources = get_playlist(
                &"https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS"
                    .to_string(),
            )
            .unwrap();

            assert!(sources.len() > 0);
            for source in sources.iter() {
                assert!(!source.url.is_empty())
            }
        }

        #[test]
        fn success_video() {
            let sources = get_playlist(
                &"https://www.youtube.com/watch?v=nBpgoga0FZ4&list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS".to_string()
            )
            .unwrap();

            assert!(sources.len() > 0);
            for source in sources.iter() {
                assert!(!source.url.is_empty())
            }
        }

        #[test]
        fn fail_video_url() {
            let sources =
                get_playlist(&"https://www.youtube.com/watch?v=6YBDo5S8soo".to_string()).unwrap();

            assert_eq!(sources.len(), 1);
        }

        #[test]
        fn fail_not_url() {
            let sources = get_playlist(&"amogus".to_string()).unwrap();

            assert!(sources.is_empty());
        }
    }

    mod timestamp {
        use crate::metadata::is_playlist;

        use super::super::parse_timestamps;

        #[test]
        fn single_line() {
            let timestamps = parse_timestamps("3:23 My description".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);
        }

        #[test]
        fn mid_line() {
            let timestamps =
                parse_timestamps("Description in front 5:55 Description behind".to_string());

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
                parse_timestamps("3:23 My description\n1:06:34 Other desc".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);

            assert_eq!(timestamps[1].label, "Other desc");
            assert_eq!(timestamps[1].timestamp, "1:06:34");
            assert_eq!(timestamps[1].seconds, 1 * 3600 + 6 * 60 + 34);
        }

        #[test]
        fn empty() {
            let timestamps = parse_timestamps("".to_string());

            assert_eq!(timestamps.len(), 0);
        }

        #[test]
        fn no_timestamps() {
            let timestamps = parse_timestamps("Hi there\n23 susser\nimposter".to_string());

            assert_eq!(timestamps.len(), 0);
        }

        #[test]
        fn is_playlist_playlist_page() {
            assert!(is_playlist(
                &"https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS"
                    .to_string()
            ))
        }

        #[test]
        fn is_playlist_video_page() {
            assert!(is_playlist(&"https://www.youtube.com/watch?v=nBpgoga0FZ4&list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS".to_string()))
        }

        #[test]
        fn is_playlist_not_video() {
            assert!(!is_playlist(
                &"https://www.youtube.com/watch?v=6YBDo5S8soo".to_string()
            ))
        }
    }
}
