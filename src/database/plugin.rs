use serenity::{
    client::{ClientBuilder, Context},
    model::prelude::UserId,
    prelude::TypeMapKey,
};

use std::{fmt::Display, sync::Arc};

use crate::media::media_info::MediaInfo;

pub struct DatabasePluginKey;

impl TypeMapKey for DatabasePluginKey {
    type Value = Arc<dyn DatabasePlugin>;
}

#[derive(Debug)]
pub struct DBError {
    pub message: String,
}

impl From<&str> for DBError {
    fn from(str: &str) -> Self {
        DBError {
            message: str.to_string(),
        }
    }
}

impl From<String> for DBError {
    fn from(str: String) -> Self {
        DBError { message: str }
    }
}

impl Display for DBError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub type PluginResult = Result<(), DBError>;
pub type PluginDataResult = Result<(Vec<MediaInfo>, usize), DBError>;

pub trait DatabasePlugin: Sync + Send {
    fn init_db(&self);

    fn disabled(&self) -> bool;

    fn set_history(&self, user_id: UserId, song: &MediaInfo) -> PluginResult;
    /// Returns the history. Latest song is index 0.
    fn get_history(&self, user_id: UserId, amount: usize, offset: usize) -> PluginDataResult;

    fn create_playlist(&self, user_id: UserId, name: &String) -> PluginResult;
    fn delete_playlist(&self, user_id: UserId, name: &String) -> PluginResult;

    fn get_playlist(
        &self,
        user_id: UserId,
        name: &String,
        amount: usize,
        offset: usize,
    ) -> PluginDataResult;

    fn get_playlists(
        &self,
        user_id: UserId,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<String>, usize), DBError>;

    fn add_playlist_songs(
        &self,
        user_id: UserId,
        name: &String,
        song: Vec<&MediaInfo>,
    ) -> PluginResult;

    fn delete_playlist_song(&self, user_id: UserId, name: &String, url: &String) -> PluginResult;
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

pub async fn get_db_plugin(ctx: &Context) -> Option<Arc<dyn DatabasePlugin>> {
    let data = ctx.data.read().await;

    data.get::<DatabasePluginKey>().cloned()
}
