use serde::{Deserialize, Serialize};
use std::{collections::LinkedList, process};
use tracing::error;

use super::media_info::{MediaInfo, PlaylistInfo};

const YOUTUBE_DL_COMMAND: &str = "yt-dlp";

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
    match process::Command::new(YOUTUBE_DL_COMMAND)
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
                    error!("[metadata] [youtube-dl] [json parse error] {}", output_str);
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

    match process::Command::new(YOUTUBE_DL_COMMAND)
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

    match process::Command::new(YOUTUBE_DL_COMMAND)
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

#[cfg(test)]
mod tests {
    mod search {
        use super::super::get_search;

        #[test]
        fn success() {
            let video = get_search(&"hello".to_string()).unwrap();

            assert!(!video.url.is_empty());
        }
    }

    mod playlist {
        use super::super::{get_playlist, is_playlist};

        #[test]
        fn success_page() {
            let sources = get_playlist(
                &"https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS"
                    .to_string(),
            )
            .unwrap();

            assert!(sources.len() > 0);
            for source in sources {
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
            for source in sources {
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
