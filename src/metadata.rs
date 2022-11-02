use crate::media::MediaInfo;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, LinkedList},
    process, vec,
};
use tracing::error;

#[derive(Serialize, Deserialize)]
struct YoutubeDLJson {
    id: Option<String>,
    url: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<f64>,
    thumbnail: Option<String>,
    webpage_url: Option<String>,
    // Expected Nullables
    playlist_title: Option<String>, // or playlist?
}

#[derive(Serialize, Deserialize)]
struct YoutubeDLFlatJson {
    ie_key: Option<String>,
    id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    duration: Option<f64>,
}

impl From<YoutubeDLJson> for MediaInfo {
    fn from(json: YoutubeDLJson) -> Self {
        MediaInfo {
            url: json.webpage_url.unwrap_or_default(),
            title: json.title.unwrap_or_default(),
            description: json.description.unwrap_or_default(),
            duration: json.duration.unwrap_or_default() as i64,
            metadata: HashMap::new(),
            thumbnail: json.thumbnail.unwrap_or_default(),
        }
    }
}

impl From<YoutubeDLFlatJson> for MediaInfo {
    fn from(json: YoutubeDLFlatJson) -> Self {
        let platform = json.ie_key.unwrap_or_default();

        let url = if platform == "Youtube" {
            "www.youtube.com/watch?v=".to_string() + &json.id.unwrap_or_default()
        } else {
            "".to_string()
        };

        MediaInfo {
            url,
            title: json.title.unwrap_or_default(),
            duration: json.duration.unwrap_or_default() as i64,
            description: json.description.unwrap_or_default(),
            metadata: HashMap::new(),
            thumbnail: "".to_string(), // FIXME
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
    match process::Command::new("youtube-dl")
        .arg("-j")
        .arg("--flat-playlist")
        .arg(&url)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let mut sources: LinkedList<MediaInfo> = LinkedList::new();

            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                error!("[playlist] [youtube-dl] {}", err_str);
            }

            let lines = output_str.split("\n");

            for line in lines {
                if line.is_empty() {
                    continue;
                }

                let json_result: serde_json::Result<YoutubeDLFlatJson> = serde_json::from_str(line);

                match json_result {
                    Ok(json) => sources.push_back(MediaInfo::from(json)),
                    Err(err) => error!("[playlist] [youtube-dl] {}", err),
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

pub fn get_videos_metadata(urls: Vec<String>) -> Result<Vec<Option<MediaInfo>>, String> {
    if urls.is_empty() {
        return Ok(vec![]);
    }

    match process::Command::new("youtube-dl")
        .arg("-j")
        .args(urls)
        .output()
    {
        Err(_) => return Err("Failed to run youtube-dl".to_string()),
        Ok(output) => {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let err_str = String::from_utf8_lossy(&output.stderr);

            if !err_str.is_empty() {
                error!("[metadata] [youtube-dl] {}", err_str);
            }

            Ok(output_str
                .split("\n")
                .filter(|s| !s.is_empty())
                .map(|s| match serde_json::from_str::<YoutubeDLJson>(s) {
                    Ok(json) => Some(MediaInfo::from(json)),
                    Err(err) => {
                        error!("{}", err);
                        None
                    }
                })
                .collect())
        }
    }
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
