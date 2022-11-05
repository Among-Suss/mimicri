use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    pub url: String,
    pub title: String,
    pub duration: i64,
    pub description: String,
    pub thumbnail: String,
}

impl MediaInfo {
    pub fn empty() -> MediaInfo {
        MediaInfo {
            url: "".to_string(),
            title: "".to_string(),
            duration: 0,
            description: "".to_string(),
            thumbnail: "".to_string(),
        }
    }
}
