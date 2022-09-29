use serenity::{
    client::{ClientBuilder, Context},
    prelude::TypeMapKey,
};

use std::sync::Arc;

pub struct DatabasePluginKey;

impl TypeMapKey for DatabasePluginKey {
    type Value = Arc<dyn DatabasePlugin>;
}

pub type DBError = &'static str;

pub trait DatabasePlugin: Sync + Send {
    fn init_db(&self);

    fn history_set(&self, user_id: i64, url: &String) -> Result<(), DBError>;
    fn history_get(&self, user_id: i64, amount: u32, page: u32) -> Result<Vec<String>, DBError>;

    fn playlist_create(&self, user_id: i64, name: &String) -> Result<(), DBError>;
    fn playlist_remove(&self, user_id: i64, name: &String) -> Result<(), DBError>;

    fn playlist_get(
        &self,
        user_id: i64,
        name: &String,
        amount: u32,
        page: u32,
    ) -> Result<Vec<String>, DBError>;

    fn playlist_song_add(&self, user_id: i64, name: &String, url: &String) -> Result<(), DBError>;
    fn playlist_song_remove(
        &self,
        user_id: i64,
        name: &String,
        url: &String,
    ) -> Result<(), DBError>;
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
