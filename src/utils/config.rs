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
        Colour::DARK_GREEN
    }

    pub fn queue() -> Colour {
        Colour::DARK_GREEN
    }

    pub fn now_playing() -> Colour {
        Colour::DARK_GREEN
    }

    pub fn history() -> Colour {
        Colour::PURPLE
    }

    pub fn playlist() -> Colour {
        Colour::PURPLE
    }

    pub fn error() -> Colour {
        Colour::RED
    }

    pub fn warn() -> Colour {
        Colour::ORANGE
    }

    pub fn info() -> Colour {
        Colour::BLUE
    }
}
