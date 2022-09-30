use crate::media::{MediaInfo, MediaItem};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, LinkedList},
    process,
};

#[derive(Serialize, Deserialize)]
struct YoutubeDLJson {
    _type: Option<String>,
    ie_key: Option<String>,
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<i64>,
}

impl From<YoutubeDLJson> for MediaInfo {
    fn from(json: YoutubeDLJson) -> Self {
        MediaInfo {
            url: format!(
                "https://www.youtube.com/watch?v={}",
                json.id.unwrap_or("".to_string())
            ), // TODO make this work with soundcloud too
            title: json.title.unwrap_or("".to_string()),
            description: json.description.unwrap_or("".to_string()),
            duration: json.duration.unwrap_or_default(),
            metadata: HashMap::new(),
        }
    }
}

pub fn get_info(url: &String) -> Result<MediaInfo, String> {
    match process::Command::new("youtube-dl")
        .arg("-j")
        .arg(url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                println!("[metadata] [youtube-dl] {}", err_str);
            }

            let json_result: serde_json::Result<YoutubeDLJson> = serde_json::from_str(&output_str);

            match json_result {
                Err(_) => Err("[metadata] [youtube-dl] Unable to parse json".to_string()),
                Ok(json) => {
                    if json.id.is_none() {
                        return Err("[metadata] [youtube-dl] ID is none".to_string());
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

pub fn get_playlist_sources(url: &String) -> Result<LinkedList<MediaInfo>, String> {
    match process::Command::new("youtube-dl")
        .arg("--flat-playlist")
        .arg("-j")
        .arg(&url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let mut sources: LinkedList<MediaInfo> = LinkedList::new();

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

                let json_result: serde_json::Result<YoutubeDLJson> = serde_json::from_str(line);

                if let Ok(json) = json_result {
                    if json.id.is_some() {
                        sources.push_back(MediaInfo::from(json));
                    }
                }
            }

            return Ok(sources);
        }
    }
}

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

            timestamps.push(Timestamp {
                seconds,
                label: line[reg_match.end()..line.len()].trim().to_string(),
                timestamp: timestamp_string.to_string(),
            })
        }
    }

    timestamps
}

pub fn get_videos_metadata(urls: Vec<String>) -> Vec<MediaItem> {
    vec![]
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
        use super::super::get_playlist_sources;

        #[test]
        fn success_page() {
            let sources = get_playlist_sources(
                &"https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS"
                    .to_string(),
            )
            .unwrap();

            for source in sources.iter() {
                assert!(!source.url.is_empty())
            }
        }

        #[test]
        fn success_video() {
            let sources = get_playlist_sources(
                &"https://www.youtube.com/watch?v=nBpgoga0FZ4&list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS".to_string()
            )
            .unwrap();

            for source in sources.iter() {
                assert!(!source.url.is_empty())
            }
        }

        #[test]
        fn fail_video_url() {
            let sources =
                get_playlist_sources(&"https://www.youtube.com/watch?v=6YBDo5S8soo".to_string())
                    .unwrap();

            assert_eq!(sources.len(), 1);
        }

        #[test]
        fn fail_not_url() {
            let sources = get_playlist_sources(&"amogus".to_string()).unwrap();

            assert!(sources.is_empty());
        }
    }

    mod timestamp {
        use super::super::get_timestamps;

        #[test]
        fn single_line() {
            let timestamps = get_timestamps("3:23 My description".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);
        }

        #[test]
        fn mid_line() {
            let timestamps = get_timestamps("Words to ignore 5:55 Some description".to_string());

            assert_eq!(timestamps[0].label, "Some description");
            assert_eq!(timestamps[0].timestamp, "5:55");
            assert_eq!(timestamps[0].seconds, 5 * 60 + 55);
        }

        #[test]
        fn multi_line() {
            let timestamps = get_timestamps("3:23 My description\n1:06:34 Other desc".to_string());

            assert_eq!(timestamps[0].label, "My description");
            assert_eq!(timestamps[0].timestamp, "3:23");
            assert_eq!(timestamps[0].seconds, 3 * 60 + 23);

            assert_eq!(timestamps[1].label, "Other desc");
            assert_eq!(timestamps[1].timestamp, "1:06:34");
            assert_eq!(timestamps[1].seconds, 1 * 3600 + 6 * 60 + 34);
        }

        #[test]
        fn empty() {
            let timestamps = get_timestamps("".to_string());

            assert_eq!(timestamps.len(), 0);
        }

        #[test]
        fn no_timestamps() {
            let timestamps = get_timestamps("Hi there\n23 susser\nimposter".to_string());

            assert_eq!(timestamps.len(), 0);
        }
    }
}
