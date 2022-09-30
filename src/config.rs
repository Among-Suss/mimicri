use serenity::model::prelude::GuildId;

static QUEUE_TEXT_LENGTH: usize = 40;
static QUEUE_PAGE_SIZE: usize = 10;

pub fn queue_text_length(_guild_id: GuildId) -> usize {
    QUEUE_TEXT_LENGTH
}

pub fn queue_page_size(_guild_id: GuildId) -> usize {
    QUEUE_PAGE_SIZE
}
