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
    fn get_connection(&self) -> Result<sqlite::Connection, DBError> {
        match sqlite::Connection::open_with_flags(
            self.path.clone(),
            OpenFlags::new()
                .set_create()
                .set_read_write()
                .set_full_mutex(),
        ) {
            Err(err) => {
                println!("[sqlite] {}", err.to_string());
                Err("Unable to connect to database")
            }
            Ok(mut c) => {
                match c.set_busy_timeout(10000) {
                    Ok(_) => (),
                    Err(err) => {
                        println!("[sqlite] Failed to set busy_timeout: {}", err.to_string())
                    }
                }
                Ok(c)
            }
        }
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
                panic!("[sqlite] Unable to connect: {}", err.to_string());
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

    fn create_playlist(&self, user_id: i64, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
                BEGIN TRANSACTION;
                INSERT OR IGNORE INTO {USER_TABLE} VALUES ({});
                INSERT OR IGNORE INTO {PLAYLIST_TABLE} VALUES (NULL, '{}', {});
                COMMIT;
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

    fn delete_playlist(&self, user_id: i64, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
                PRAGMA foreign_keys = ON;
                DELETE FROM {PLAYLIST_TABLE} WHERE name='{}' AND user_id = {};
                ",
            name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                println!("[sqlite] {}", err.to_string());
                return Err("Failed to delete song");
            }
        }

        Ok(())
    }

    fn add_playlist_song(&self, user_id: i64, name: &String, url: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

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
                println!("[sqlite] {}", err.to_string());
                return Err("Failed to set history");
            }
        }

        Ok(())
    }

    fn delete_playlist_song(
        &self,
        user_id: i64,
        name: &String,
        url: &String,
    ) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
                PRAGMA foreign_keys = ON;

                DELETE FROM {PLAYLIST_MAP_TABLE}
                WHERE song_url='{}' AND 
                      id=(
                        SELECT id 
                        FROM {PLAYLIST_TABLE}
                        WHERE name='{}' AND
                              user_id = {}
                      );
                ",
            url, name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                println!("[sqlite] {}", err.to_string());
                return Err("Failed to delete song");
            }
        }

        Ok(())
    }

    fn get_playlist(
        &self,
        user_id: i64,
        name: &String,
        amount: u32,
        page: u32,
    ) -> Result<Vec<String>, DBError> {
        let connection = self.get_connection()?;

        let query = format!(
            "
            SELECT song_url
            FROM {PLAYLIST_MAP_TABLE} 
            WHERE playlist_id=(
                SELECT id
                FROM {PLAYLIST_TABLE}
                WHERE user_id={} AND name='{}'
            )
            ORDER BY id
            LIMIT {}
            OFFSET {}
            ;
        ",
            user_id, name, amount, page
        );

        let mut cursor = connection.prepare(query).unwrap().into_cursor();

        let mut urls: Vec<String> = Vec::new();

        while let Some(Ok(row)) = cursor.next() {
            urls.push(row.get::<String, _>(0));
        }

        Ok(urls)
    }

    fn set_history(&self, user_id: i64, url: &String) -> Result<(), DBError> {
        self.add_playlist_song(user_id, &HISTORY_PLAYLIST.to_string(), url)
    }

    fn get_history(&self, user_id: i64, amount: u32, page: u32) -> Result<Vec<String>, DBError> {
        self.get_playlist(user_id, &HISTORY_PLAYLIST.to_string(), amount, page)
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    const TEST_DB: &str = "test.sqlite";

    fn mock_db_plugin() -> SQLitePlugin {
        let mut connection = sqlite::Connection::open_with_flags(
            TEST_DB,
            OpenFlags::new()
                .set_create()
                .set_read_write()
                .set_full_mutex(),
        )
        .unwrap();

        let _ = connection.set_busy_timeout(10000);

        connection
            .execute(
                "
            DROP TABLE IF EXISTS users;
            DROP TABLE IF EXISTS songs;
            DROP TABLE IF EXISTS playlists;
            DROP TABLE IF EXISTS playlists_map;
        
        ",
            )
            .unwrap();

        let plugin = SQLitePlugin {
            path: TEST_DB.to_string(),
        };
        plugin.init_db();
        plugin
    }

    #[test]
    #[serial]
    fn disabled() {
        let plugin = SQLitePlugin {
            path: "".to_string(),
        };
        plugin.init_db();

        assert!(plugin.set_history(1, &"url".to_string()).is_ok());
    }

    #[test]
    #[serial]
    fn set_history() {
        let db = mock_db_plugin();

        let user_id = 1;
        let song_url = "url1".to_string();

        db.set_history(user_id, &song_url).unwrap();

        let connection = db.get_connection().unwrap();

        let user_row = connection
            .prepare("SELECT * FROM users")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        let song_row = connection
            .prepare("SELECT * FROM songs")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        let playlist_row = connection
            .prepare("SELECT * FROM playlists")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        let playlist_map = connection
            .prepare("SELECT * FROM playlists_map")
            .unwrap()
            .into_cursor()
            .next()
            .unwrap()
            .unwrap();

        assert_eq!(user_row.get::<i64, _>(0), user_id);
        assert_eq!(song_row.get::<String, _>(0), song_url);

        assert_eq!(playlist_row.get::<String, _>(1), "_history");
        assert_eq!(playlist_row.get::<i64, _>(2), user_id);

        assert_eq!(playlist_map.get::<String, _>(2), song_url);
    }

    #[test]
    #[serial]
    fn get_history() {
        let db = mock_db_plugin();

        let user_id = 1;
        let song_1 = "url1".to_string();
        let song_2 = "url2".to_string();
        let song_3 = "url3".to_string();
        let song_4 = "url4".to_string();
        let song_5 = "url5".to_string();

        db.set_history(user_id, &song_1).unwrap();
        db.set_history(user_id, &song_2).unwrap();
        db.set_history(user_id, &song_3).unwrap();
        db.set_history(user_id, &song_4).unwrap();
        db.set_history(user_id, &song_5).unwrap();

        let history = db.get_history(user_id, 5, 0).unwrap();

        assert_eq!(song_1, history[0]);
        assert_eq!(song_2, history[1]);
        assert_eq!(song_3, history[2]);
        assert_eq!(song_4, history[3]);
        assert_eq!(song_5, history[4]);
        assert_eq!(history.len(), 5);
    }

    #[test]
    #[serial]
    fn get_history_pagination() {
        let db = mock_db_plugin();

        let user_id = 1;
        let song_1 = "url1".to_string();
        let song_2 = "url2".to_string();
        let song_3 = "url3".to_string();
        let song_4 = "url4".to_string();
        let song_5 = "url5".to_string();

        db.set_history(user_id, &song_1).unwrap();
        db.set_history(user_id, &song_2).unwrap();
        db.set_history(user_id, &song_3).unwrap();
        db.set_history(user_id, &song_4).unwrap();
        db.set_history(user_id, &song_5).unwrap();

        let history = db.get_history(user_id, 5, 2).unwrap();

        assert_eq!(song_3, history[0]);
        assert_eq!(song_4, history[1]);
        assert_eq!(song_5, history[2]);
        assert_eq!(history.len(), 3);
    }

    #[test]
    #[serial]
    fn add_and_delete_song() {
        let db = mock_db_plugin();

        let user_id = 1;
        let playlist_name = "playlist".to_string();
        let url = "sussy url".to_string();

        db.add_playlist_song(user_id, &playlist_name, &url).unwrap();
        db.add_playlist_song(user_id, &playlist_name, &"extra_song1".to_string())
            .unwrap();
        db.add_playlist_song(user_id, &playlist_name, &"extra_song2".to_string())
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap();

        assert_eq!(songs[0], url);
        assert_eq!(songs.len(), 3);

        db.delete_playlist_song(user_id, &playlist_name, &url)
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap();

        assert_eq!(songs.len(), 2);
    }

    #[test]
    #[serial]
    fn delete_playlist() {
        let db = mock_db_plugin();

        let user_id = 5;
        let playlist_name = "amogus twerking compilation".to_string();

        db.add_playlist_song(user_id, &playlist_name, &"song_1".to_string())
            .unwrap();
        db.add_playlist_song(user_id, &playlist_name, &"song_2".to_string())
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 10, 0).unwrap();

        assert_eq!(songs.len(), 2);

        db.delete_playlist(user_id, &playlist_name).unwrap();

        let connection = db.get_connection().unwrap();

        let mut cursor = connection
            .prepare(format!("SELECT COUNT(*) FROM {PLAYLIST_MAP_TABLE};"))
            .unwrap()
            .into_cursor();

        assert_eq!(cursor.next().unwrap().unwrap().get::<i64, _>(0), 0);
    }
}
