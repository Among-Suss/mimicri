use std::env;

use sqlite::OpenFlags;

use super::plugin::{DBError, DatabasePlugin};

const USER_TABLE: &str = "users";
const SONG_TABLE: &str = "songs";
const PLAYLIST_TABLE: &str = "playlists";
const PLAYLIST_MAP_TABLE: &str = "playlists_map";

const HISTORY_PLAYLIST: &str = "_history";

pub struct SQLitePlugin {
    pub path: String,
}

impl SQLitePlugin {
    fn get_connection(&self) -> Result<sqlite::Connection, sqlite::Error> {
        sqlite::Connection::open_with_flags(
            self.path.clone(),
            OpenFlags::new()
                .set_create()
                .set_read_write()
                .set_full_mutex(),
        )
    }

    fn is_disabled(&self) -> bool {
        self.path.eq("")
    }
}

impl Default for SQLitePlugin {
    fn default() -> Self {
        let db = env::var("PLUGIN_SQLITE").unwrap_or("".to_string());

        if db.eq("") {
            println!(
                "[sqlite] sqlite plugin disabled. Set PLUGIN_SQLITE to a file name like 'mimicri.db' to enable the plugin.");
        }

        SQLitePlugin { path: db }
    }
}

impl DatabasePlugin for SQLitePlugin {
    fn init_db(&self) {
        if self.is_disabled() {
            return;
        }

        let connection = match self.get_connection() {
            Err(err) => {
                println!("[sqlite] {}", err.message.unwrap_or_default());
                return;
            }
            Ok(c) => c,
        };

        connection
            .execute(format!(
                "
                CREATE TABLE IF NOT EXISTS {USER_TABLE} (
                    id INTEGER PRIMARY KEY

                );
                CREATE TABLE IF NOT EXISTS {SONG_TABLE} (
                    url TEXT PRIMARY KEY
                );
                CREATE TABLE IF NOT EXISTS {PLAYLIST_TABLE} (
                    id INTEGER PRIMARY KEY,

                    name TEXT NOT NULL,
                    user_id INTEGER,

                    CONSTRAINT un
                        UNIQUE (name, user_id),
                    
                    CONSTRAINT del_users
                        FOREIGN KEY(user_id) REFERENCES {USER_TABLE}(id)
                        ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS {PLAYLIST_MAP_TABLE} (
                    id INTEGER PRIMARY KEY,

                    playlist_id INTEGER,
                    song_url TEXT,

                    CONSTRAINT un
                        UNIQUE (playlist_id, song_url),

                    CONSTRAINT del_playlists
                        FOREIGN KEY(playlist_id) REFERENCES {PLAYLIST_TABLE}(id)
                        ON DELETE CASCADE,

                    CONSTRAINT del_songs
                        FOREIGN KEY(song_url) REFERENCES {SONG_TABLE}(url)
                        ON DELETE CASCADE
                );
                "
            ))
            .expect("[sqlite] Unable to init database");
    }

    fn history_set(&self, user_id: u64, url: String) -> Result<(), &'static str> {
        self.playlist_song_add(user_id, HISTORY_PLAYLIST.to_string(), url)
    }

    fn history_get(&self, user_id: u64, url: String) -> Result<Vec<String>, &'static str> {
        todo!()
    }

    fn playlist_create(&self, user_id: u64, name: String) -> Result<(), &'static str> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = match self.get_connection() {
            Ok(c) => c,
            Err(err) => {
                println!("[sqlite] {}", err);
                return Err("Unable to connect to database");
            }
        };

        let query = format!(
            "
                INSERT OR IGNORE INTO {USER_TABLE} VALUES ({});
                INSERT OR IGNORE INTO {PLAYLIST_TABLE} VALUES (NULL, '{}', {});

                ",
            user_id, name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                println!("[sqlite] Failed to create playlist {}; {}", name, err);
                return Err("Failed to create playlist");
            }
        }

        Ok(())
    }

    fn playlist_remove(&self, user_id: u64, name: String) -> Result<(), &'static str> {
        if self.is_disabled() {
            return Ok(());
        }

        todo!()
    }

    fn playlist_song_add(
        &self,
        user_id: u64,
        name: String,
        url: String,
    ) -> Result<(), &'static str> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = match self.get_connection() {
            Ok(c) => c,
            Err(err) => {
                println!("[sqlite] {}", err);
                return Err("Unable to connect to database");
            }
        };

        let query = format!(
            "
            BEGIN TRANSACTION;
                INSERT OR IGNORE INTO {USER_TABLE} VALUES ({});
                INSERT OR IGNORE INTO {SONG_TABLE} VALUES ('{}');
                INSERT OR IGNORE INTO {PLAYLIST_TABLE} VALUES (NULL, '{}', {});
                INSERT OR IGNORE INTO {PLAYLIST_MAP_TABLE} VALUES (NULL, (
                    SELECT id FROM {PLAYLIST_TABLE} WHERE name='{}' AND user_id={} LIMIT 1
                ), '{}');
            COMMIT;
                ",
            user_id, url, name, user_id, name, user_id, url
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                println!("[sqlite] {}", err);
                return Err("Failed to set history");
            }
        }

        Ok(())
    }

    fn playlist_song_remove(
        &self,
        user_id: u64,
        name: String,
        url: String,
    ) -> Result<(), &'static str> {
        if self.is_disabled() {
            return Ok(());
        }

        todo!()
    }

    fn playlist_get(&self, user_id: u64, name: String) -> Result<Vec<String>, DBError> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::remove_file;

    use super::*;

    const TEST_DB: &str = "test.db";

    fn mock_db_plugin() -> SQLitePlugin {
        let _ = remove_file(TEST_DB);

        let plugin = SQLitePlugin {
            path: TEST_DB.to_string(),
        };
        plugin.init_db();
        plugin
    }

    #[test]
    fn disabled() {
        let plugin = SQLitePlugin {
            path: "".to_string(),
        };
        plugin.init_db();

        assert!(plugin.history_set(1, "url".to_string()).is_ok());
    }

    #[test]
    fn set_history() {
        let db = mock_db_plugin();

        db.history_set(1, "url1".to_string()).unwrap();

        let connection = db.get_connection().unwrap();

        let user_row = connection
            .prepare("SELECT * FROM users")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        let playlist_row = connection
            .prepare("SELECT * FROM songs")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        assert_eq!(user_row.get::<i64, _>(0), 1);
        assert_eq!(playlist_row.get::<String, _>(0), "url1");
    }

    #[test]
    fn get_history() {}
}
