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
                println!("[playlist] youtube-dl error: {}", err_str);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_page() {
        let sources = get_playlist_sources(
            "https://www.youtube.com/playlist?list=PLdY_Mca8fL_BbtQrKu9lm-LcCcY-t2mVS".to_string(),
        )
        .unwrap();

        for source in sources.iter() {
            assert!(!source.is_empty())
        }
    }

    #[test]
    fn test_fail_video_url() {
        let sources =
            get_playlist_sources("https://www.youtube.com/watch?v=6YBDo5S8soo".to_string())
                .unwrap();

        assert_eq!(sources.len(), 1);
    }

    #[test]
    fn test_fail_not_url() {
        let sources = get_playlist_sources("amogus".to_string()).unwrap();

        assert!(sources.is_empty());
    }
}
