use std::env;

use async_std::fs;

pub fn get_log_filename() -> String {
    env::var("LOG_FILE").unwrap_or("output.log".to_owned())
}

pub async fn get_log() -> Result<Vec<String>, String> {
    if let Ok(file_data) = fs::read_to_string(get_log_filename()).await {
        Ok(file_data.split("\n").map(|s| s.to_owned()).collect())
    } else {
        Err("Unable to get file".to_owned())
    }
}
