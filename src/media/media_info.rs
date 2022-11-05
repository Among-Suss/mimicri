use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct MediaInfo {
    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub title: String,

    #[serde(default)]
    pub duration: i64,

    #[serde(default)]
    pub description: String,

    #[serde(default)]
    pub thumbnail: String,

    #[serde(default)]
    pub uploader: String,

    #[serde(default)]
    pub playlist: Option<PlaylistInfo>,
}

impl MediaInfo {
    pub fn empty() -> MediaInfo {
        MediaInfo {
            url: "".to_string(),
            title: "".to_string(),
            duration: 0,
            description: "".to_string(),
            thumbnail: "".to_string(),
            uploader: "".to_string(),
            playlist: None,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct PlaylistInfo {
    pub title: String,
    pub uploader: String,
}
