use std::env;

use sqlite::OpenFlags;
use tracing::{error, info};

use crate::media::MediaInfo;

use super::plugin::{DBError, DatabasePlugin};

const HISTORY_PLAYLIST: &str = "_history";

fn escape_json(json: String) -> String {
    json.replace("'", "''")
}

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
                error!("[sqlite] {}", err.to_string());
                Err("Unable to connect to database".to_string())
            }
            Ok(mut c) => {
                match c.set_busy_timeout(10000) {
                    Ok(_) => (),
                    Err(err) => {
                        error!("[sqlite] Failed to set busy_timeout: {}", err.to_string())
                    }
                }
                Ok(c)
            }
        }
    }

    fn is_disabled(&self) -> bool {
        self.path.eq("")
    }

    fn _get_playlist(
        &self,
        user_id: u64,
        name: &String,
        amount: usize,
        offset: usize,
        reverse: bool,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        if self.is_disabled() {
            return Err("SQLite plugin not enabled!".to_string());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
            SELECT playlists_map.song_url, songs.metadata 
            FROM playlists_map 
            INNER JOIN songs ON songs.url=playlists_map.song_url
            WHERE playlists_map.playlist_id=(
                SELECT id
                FROM playlists
                WHERE user_id={} AND name='{}'
            )
            ORDER BY id {}
            LIMIT {}
            OFFSET {}
            ;
        ",
            user_id,
            name,
            if reverse { "DESC" } else { "ASC" },
            amount,
            offset
        );

        let mut cursor = connection.prepare(query).unwrap().into_cursor();

        let mut infos: Vec<MediaInfo> = Vec::new();

        while let Some(Ok(row)) = cursor.next() {
            let info_json = row.get::<String, _>(1);

            infos.push(
                match serde_json::from_str::<MediaInfo>(info_json.as_str()) {
                    Ok(info) => info,
                    Err(err) => {
                        error!(
                            "Unable to deserialize json from history: {}. Error message: {}",
                            info_json, err
                        );

                        let url = row.get::<String, _>(0);

                        MediaInfo {
                            url: url.clone(),
                            title: url.clone(),
                            ..MediaInfo::empty()
                        }
                    }
                },
            );
        }

        // Get total
        let query = format!(
            "
            SELECT count(*)
            FROM playlists_map 
            WHERE playlists_map.playlist_id=(
                SELECT id
                FROM playlists
                WHERE user_id={} AND name='{}'
            )
            ;
        ",
            user_id, name,
        );

        let mut cursor = connection.prepare(query).unwrap().into_cursor();

        let count = if let Some(Ok(row)) = cursor.next() {
            row.get::<i64, _>(0)
        } else {
            0
        };

        Ok((infos, count as usize))
    }
}

impl Default for SQLitePlugin {
    fn default() -> Self {
        let db = env::var("SQLITE_DB").unwrap_or("".to_string());

        if db.eq("") {
            info!(
                "[sqlite] sqlite plugin disabled. Set SQLITE_DB to a file name like 'mimicri.db' to enable the plugin.");
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
                CREATE TABLE IF NOT EXISTS users (
                    id INTEGER PRIMARY KEY

                );
                CREATE TABLE IF NOT EXISTS songs (
                    url TEXT PRIMARY KEY,
                    metadata TEXT
                );
                CREATE TABLE IF NOT EXISTS playlists (
                    id INTEGER PRIMARY KEY,

                    name TEXT NOT NULL,
                    user_id INTEGER,

                    CONSTRAINT un
                        UNIQUE (name, user_id),
                    
                    CONSTRAINT del_users
                        FOREIGN KEY(user_id) REFERENCES users(id)
                        ON DELETE CASCADE
                );
                CREATE TABLE IF NOT EXISTS playlists_map (
                    id INTEGER PRIMARY KEY,

                    playlist_id INTEGER,
                    song_url TEXT,

                    CONSTRAINT un
                        UNIQUE (playlist_id, song_url),

                    CONSTRAINT del_playlists
                        FOREIGN KEY(playlist_id) REFERENCES playlists(id)
                        ON DELETE CASCADE,

                    CONSTRAINT del_songs
                        FOREIGN KEY(song_url) REFERENCES songs(url)
                        ON DELETE CASCADE
                );
                "
            ))
            .expect("[sqlite] Unable to init database");
    }

    fn create_playlist(&self, user_id: u64, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
                BEGIN TRANSACTION;
                INSERT OR IGNORE INTO users VALUES ({});
                INSERT OR IGNORE INTO playlists VALUES (NULL, '{}', {});
                COMMIT;
                ",
            user_id, name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] Failed to create playlist {}; {}", name, err);
                return Err("Failed to create playlist".to_string());
            }
        }

        Ok(())
    }

    fn delete_playlist(&self, user_id: u64, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
                PRAGMA foreign_keys = ON;
                DELETE FROM playlists WHERE name='{}' AND user_id = {};
                ",
            name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] {}", err.to_string());
                return Err("Failed to delete song".to_string());
            }
        }

        Ok(())
    }

    fn add_playlist_song(
        &self,
        user_id: u64,
        name: &String,
        info: MediaInfo,
    ) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        let query = format!(
            "
            BEGIN TRANSACTION;
                INSERT OR IGNORE INTO users VALUES ({});
                INSERT OR IGNORE INTO songs VALUES ('{}', '{}');
                INSERT OR IGNORE INTO playlists VALUES (NULL, '{}', {});
                INSERT OR REPLACE INTO playlists_map VALUES (NULL, (
                    SELECT id FROM playlists WHERE name='{}' AND user_id={} LIMIT 1
                ), '{}');
            COMMIT;
                ",
            user_id,
            &info.url,
            escape_json(serde_json::to_value(info.clone()).unwrap().to_string()),
            name,
            user_id,
            name,
            user_id,
            &info.url
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] {}", err.to_string());
                return Err("Failed to add to playlist".to_string());
            }
        }

        Ok(())
    }

    fn delete_playlist_song(
        &self,
        user_id: u64,
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

                DELETE FROM playlists_map
                WHERE song_url='{}' AND 
                      id=(
                        SELECT id 
                        FROM playlists
                        WHERE name='{}' AND
                              user_id = {}
                      );
                ",
            url, name, user_id
        );

        match connection.execute(query) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] {}", err.to_string());
                return Err("Failed to delete song".to_string());
            }
        }

        Ok(())
    }

    fn get_playlist(
        &self,
        user_id: u64,
        name: &String,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        self._get_playlist(user_id, name, amount, offset, false)
    }

    fn set_history(&self, user_id: u64, info: MediaInfo) -> Result<(), DBError> {
        self.add_playlist_song(user_id, &HISTORY_PLAYLIST.to_string(), info)
    }

    fn get_history(
        &self,
        user_id: u64,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        self._get_playlist(user_id, &HISTORY_PLAYLIST.to_string(), amount, offset, true)
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

        assert!(plugin.set_history(1, MediaInfo::empty()).is_ok());
    }

    #[test]
    #[serial]
    fn set_playlist_escape_single_quote() {
        let db = mock_db_plugin();
        let user_id = 1;
        let playlist = "playlist".to_string();

        let song = MediaInfo {
            url: "test_url".to_string(),
            description: "It's".to_string(),
            duration: 0,
            title: "".to_string(),
            thumbnail: "".to_string(),
        };

        let result = db.add_playlist_song(user_id, &playlist, song.clone());

        assert!(result.is_ok());

        let playlist_songs = db.get_playlist(user_id, &playlist, 1, 0).unwrap().0;

        assert_eq!(playlist_songs[0].description, song.description);
    }

    #[test]
    #[serial]
    fn set_playlist_escape_tokens() {
        let db = mock_db_plugin();
        let user_id = 1;
        let playlist = "playlist".to_string();

        let song = MediaInfo {
            url: "test_url".to_string(),
            description: "');".to_string(),
            duration: 0,
            title: "".to_string(),
            thumbnail: "".to_string(),
        };

        let result = db.add_playlist_song(user_id, &playlist, song.clone());

        assert!(result.is_ok());

        let playlist_songs = db.get_playlist(user_id, &playlist, 1, 0).unwrap().0;

        assert_eq!(playlist_songs[0].description, song.description);
    }

    #[test]
    #[serial]
    fn set_history() {
        let db = mock_db_plugin();

        let user_id = 1;

        let song = mock_info("url1");

        db.set_history(user_id, song.clone()).unwrap();

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

        assert_eq!(user_row.get::<i64, _>(0), user_id as i64);
        assert_eq!(song_row.get::<String, _>(0), song.url);

        assert_eq!(playlist_row.get::<String, _>(1), "_history");
        assert_eq!(playlist_row.get::<i64, _>(2), user_id as i64);

        assert_eq!(playlist_map.get::<String, _>(2), song.url);
    }

    fn mock_info(url: &str) -> MediaInfo {
        MediaInfo {
            url: url.to_string(),
            ..MediaInfo::empty()
        }
    }

    #[test]
    #[serial]
    fn get_history_reversed() {
        let db = mock_db_plugin();

        let user_id = 1;
        let song_1 = mock_info("url1");
        let song_2 = mock_info("url2");
        let song_3 = mock_info("url3");
        let song_4 = mock_info("url4");
        let song_5 = mock_info("url5");

        db.set_history(user_id, song_1.clone()).unwrap();
        db.set_history(user_id, song_2.clone()).unwrap();
        db.set_history(user_id, song_3.clone()).unwrap();
        db.set_history(user_id, song_4.clone()).unwrap();
        db.set_history(user_id, song_5.clone()).unwrap();

        let history = db.get_history(user_id, 5, 0).unwrap().0;

        assert_eq!(song_1, history[4]);
        assert_eq!(song_2, history[3]);
        assert_eq!(song_3, history[2]);
        assert_eq!(song_4, history[1]);
        assert_eq!(song_5, history[0]);
        assert_eq!(history.len(), 5);
    }

    #[test]
    #[serial]
    fn get_history_pagination() {
        let db = mock_db_plugin();

        let user_id = 1;
        let song_1 = mock_info("url1");
        let song_2 = mock_info("url2");
        let song_3 = mock_info("url3");
        let song_4 = mock_info("url4");
        let song_5 = mock_info("url5");

        db.set_history(user_id, song_1.clone()).unwrap();
        db.set_history(user_id, song_2.clone()).unwrap();
        db.set_history(user_id, song_3.clone()).unwrap();
        db.set_history(user_id, song_4).unwrap();
        db.set_history(user_id, song_5).unwrap();

        let query = db.get_history(user_id, 5, 2).unwrap();
        let history = query.0;
        let count = query.1;

        assert_eq!(song_3, history[0]);
        assert_eq!(song_2, history[1]);
        assert_eq!(song_1, history[2]);
        assert_eq!(count, 5);
    }

    #[test]
    #[serial]
    fn add_and_delete_song() {
        let db = mock_db_plugin();

        let user_id = 1;
        let playlist_name = "playlist".to_string();
        let song1 = mock_info("sussy_url");
        let song2 = mock_info("extra_song_1");
        let song3 = mock_info("extra_song_2");

        db.add_playlist_song(user_id, &playlist_name, song1.clone())
            .unwrap();
        db.add_playlist_song(user_id, &playlist_name, song2.clone())
            .unwrap();
        db.add_playlist_song(user_id, &playlist_name, song3.clone())
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap().0;

        assert_eq!(songs[0], song1);
        assert_eq!(songs.len(), 3);

        db.delete_playlist_song(user_id, &playlist_name, &song1.url)
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap().0;

        assert_eq!(songs.len(), 2);
    }

    #[test]
    #[serial]
    fn delete_playlist() {
        let db = mock_db_plugin();

        let user_id = 5;
        let playlist_name = "amogus twerking compilation".to_string();

        let song1 = mock_info("song_1");
        let song2 = mock_info("song_2");

        db.add_playlist_song(user_id, &playlist_name, song1)
            .unwrap();
        db.add_playlist_song(user_id, &playlist_name, song2)
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 10, 0).unwrap().0;

        assert_eq!(songs.len(), 2);

        db.delete_playlist(user_id, &playlist_name).unwrap();

        let connection = db.get_connection().unwrap();

        let mut cursor = connection
            .prepare(format!("SELECT COUNT(*) FROM playlists_map;"))
            .unwrap()
            .into_cursor();

        assert_eq!(cursor.next().unwrap().unwrap().get::<i64, _>(0), 0);
    }
}
