use serenity::model::prelude::GuildId;

pub fn queue_text_length(_guild_id: GuildId) -> usize {
    40
}

pub fn queue_page_size(_guild_id: GuildId) -> usize {
    10
}

pub fn progress_bar_length(_guild_id: GuildId) -> usize {
    20
}

pub fn progress_bar_marker(_guild_id: GuildId) -> String {
    "🔘".to_string()
}

pub fn progress_bar_track(_guild_id: GuildId) -> String {
    "─".to_string()
}
