use serenity::{
    client::{ClientBuilder, Context},
    prelude::TypeMapKey,
};

use std::sync::Arc;

pub struct DatabasePluginKey;

impl TypeMapKey for DatabasePluginKey {
    type Value = Arc<dyn DatabasePlugin>;
}

pub trait DatabasePlugin: Sync + Send {
    fn init_db(&self);

    fn history_set(&self, user_id: u64, url: String) -> Result<(), &'static str>;
    fn history_get(&self, user_id: u64, url: String) -> Result<Vec<String>, &'static str>;

    fn playlist_create(&self, user_id: u64, name: String) -> Result<(), &'static str>;
    fn playlist_remove(&self, user_id: u64, name: String) -> Result<(), &'static str>;
    fn playlist_song_add(
        &self,
        user_id: u64,
        name: String,
        url: String,
    ) -> Result<(), &'static str>;
    fn playlist_song_remove(
        &self,
        user_id: u64,
        name: String,
        url: String,
    ) -> Result<(), &'static str>;

    fn remove_song(&self, url: String) -> Result<(), &'static str>;
}

fn register_database_plugin(
    client_builder: ClientBuilder,
    plugin: Arc<dyn DatabasePlugin>,
) -> ClientBuilder {
    client_builder.type_map_insert::<DatabasePluginKey>(plugin)
}

pub trait DatabasePluginInit {
    fn register_database_plugin(self, plugin: Arc<dyn DatabasePlugin>) -> Self;
}

impl DatabasePluginInit for ClientBuilder {
    fn register_database_plugin(self, plugin: Arc<dyn DatabasePlugin>) -> Self {
        plugin.init_db();
        register_database_plugin(self, plugin)
    }
}

pub async fn get(ctx: &Context) -> Option<Arc<dyn DatabasePlugin>> {
    let data = ctx.data.read().await;

    data.get::<DatabasePluginKey>().cloned()
}
