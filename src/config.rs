use serenity::model::prelude::GuildId;

pub mod queue {
    use super::*;

    pub fn text_length(_guild_id: GuildId) -> usize {
        60
    }

    pub fn page_size(_guild_id: GuildId) -> usize {
        10
    }
}

pub mod progress_bar {
    use super::*;

    pub fn length(_guild_id: GuildId) -> usize {
        40
    }

    pub fn marker(_guild_id: GuildId) -> String {
        "🔘".to_string()
    }

    pub fn track(_guild_id: GuildId) -> String {
        "─".to_string()
    }
}

pub mod colors {
    use serenity::utils::Colour;

    pub fn play() -> Colour {
        Colour::from_rgb(0xf5, 0xc5, 0x05)
    }

    pub fn queue() -> Colour {
        Colour::from_rgb(0xf5, 0xc5, 0x05)
    }

    pub fn now_playing() -> Colour {
        Colour::from_rgb(0xf5, 0xc5, 0x05)
    }

    pub fn history() -> Colour {
        Colour::from_rgb(0xf5, 0xc5, 0x05)
    }

    pub fn error() -> Colour {
        Colour::RED
    }

    pub fn info() -> Colour {
        Colour::BLUE
    }
}
