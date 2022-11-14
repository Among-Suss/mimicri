use std::sync::Arc;

use poise::serenity_prelude as serenity;

use super::global_media_player::GlobalMediaPlayer;

pub struct GlobalMediaPlayerKey;

impl serenity::TypeMapKey for GlobalMediaPlayerKey {
    type Value = Arc<GlobalMediaPlayer>;
}

fn register_media_player_plugin(
    client_builder: serenity::ClientBuilder,
    plugin: Arc<GlobalMediaPlayer>,
) -> serenity::ClientBuilder {
    client_builder.type_map_insert::<GlobalMediaPlayerKey>(plugin)
}

pub trait GlobalMediaPlayerPluginInit {
    fn register_media_player_plugin(self, plugin: Arc<GlobalMediaPlayer>) -> Self;
}

impl GlobalMediaPlayerPluginInit for serenity::ClientBuilder {
    fn register_media_player_plugin(self, plugin: Arc<GlobalMediaPlayer>) -> Self {
        register_media_player_plugin(self, plugin)
    }
}

pub async fn get_media_player(ctx: &serenity::Context) -> Option<Arc<GlobalMediaPlayer>> {
    let data = ctx.data.read().await;

    data.get::<GlobalMediaPlayerKey>().cloned()
}
