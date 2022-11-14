use std::env;

use rusqlite::{named_params, params, Connection, OpenFlags};
use serenity::model::prelude::UserId;
use tracing::{error, info};

use crate::media::media_info::MediaInfo;

use super::plugin::{DBError, DatabasePlugin};

const HISTORY_PLAYLIST: &str = "_history";

impl From<rusqlite::Error> for DBError {
    fn from(err: rusqlite::Error) -> Self {
        DBError {
            message: format!("sqlite error: {}", err),
        }
    }
}

fn escape_json(json: String) -> String {
    json.replace("'", "''")
}

pub struct SQLitePlugin {
    pub path: String,
}

impl SQLitePlugin {
    fn get_connection(&self) -> Result<Connection, DBError> {
        match Connection::open_with_flags(
            &self.path,
            OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        ) {
            Ok(conn) => Ok(conn),
            Err(err) => Err(err.into()),
        }
    }

    fn is_disabled(&self) -> bool {
        self.path.eq("")
    }

    fn _get_playlist(
        &self,
        user_id: UserId,
        name: &String,
        amount: usize,
        offset: usize,
        reverse: bool,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        if self.is_disabled() {
            return Err("SQLite plugin not enabled!".into());
        }

        let connection = self.get_connection()?;

        // Get songs
        let mut statement = connection.prepare(&format!(
            "
            SELECT playlists_map.song_url, songs.metadata
            FROM playlists_map 
            INNER JOIN songs ON songs.url=playlists_map.song_url
            WHERE playlists_map.playlist_id=(
                SELECT id
                FROM playlists
                WHERE user_id=:user_id AND name=:playlist_name
            )
            ORDER BY id {}
            LIMIT :limit
            OFFSET :offset
            ",
            if reverse { "DESC" } else { "ASC" }
        ))?;

        let query = statement.query_map(
            named_params! {
                ":user_id": user_id.as_u64(),
                ":playlist_name": name,
                ":limit": amount,
                ":offset": offset,
            },
            |r| {
                let url = r.get::<_, String>(0).unwrap();
                let info_json = r.get::<_, String>(1).unwrap();
                // size = r.get::<_, i64>(2).unwrap();

                match serde_json::from_str::<MediaInfo>(info_json.as_str()) {
                    Ok(info) => Ok(info),
                    Err(err) => {
                        error!(
                            "Unable to deserialize json from history: {}. Error message: {}",
                            info_json, err
                        );
                        Ok(MediaInfo {
                            url: url.clone(),
                            title: url.clone(),
                            ..MediaInfo::empty()
                        })
                    }
                }
            },
        )?;

        let songs: Vec<MediaInfo> = query.filter_map(|m| m.ok()).collect();

        // Get size
        let mut statement = connection.prepare(
            "
            SELECT count(*)
            FROM playlists_map 
            WHERE playlists_map.playlist_id=(
                SELECT id
                FROM playlists
                WHERE user_id=?1 AND name=?2
            )
            ",
        )?;

        let size: i64 = statement
            .query(params![user_id.as_u64(), name])?
            .next()?
            .unwrap()
            .get(0)?;

        Ok((songs, size as usize))
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
            .execute_batch(
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
                ",
            )
            .expect("[sqlite] Unable to init database");
    }

    fn disabled(&self) -> bool {
        self.is_disabled()
    }

    fn create_playlist(&self, user_id: UserId, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        if name == HISTORY_PLAYLIST {
            return Err("Cannot use this name for a database.".into());
        }

        let connection = self.get_connection()?;

        connection.execute(
            "INSERT OR IGNORE INTO users VALUES (?1)",
            params![&user_id.as_u64()],
        )?;

        match connection.execute(
            "
                INSERT OR IGNORE INTO playlists VALUES (NULL, ?1, ?2)
                ",
            params![name, user_id.as_u64()],
        ) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] Failed to create playlist {}; {}", name, err);
                return Err("Failed to create playlist".into());
            }
        }

        Ok(())
    }

    fn delete_playlist(&self, user_id: UserId, name: &String) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        connection.execute("PRAGMA foreign_keys = ON", ())?;

        match connection.execute(
            "DELETE FROM playlists WHERE name=?1 AND user_id = ?2",
            (name, user_id.as_u64()),
        ) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] {}", err.to_string());
                return Err(format!("Failed to delete playlist: {}", err)
                    .as_str()
                    .into());
            }
        }

        Ok(())
    }

    fn add_playlist_songs(
        &self,
        user_id: UserId,
        name: &String,
        songs: Vec<&MediaInfo>,
    ) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        // Create user
        connection.execute(
            "INSERT OR IGNORE INTO users VALUES (?1)",
            params![&user_id.as_u64()],
        )?;

        // Create or get playlist
        let mut statement =
            connection.prepare("SELECT id FROM playlists WHERE user_id=?1 AND name=?2")?;

        let playlist_id: Option<i64> =
            match statement.query(params![&user_id.as_u64(), name])?.next()? {
                Some(row) => Some(row.get(0)?),
                None => None,
            };

        let playlist_id: i64 = match playlist_id {
            Some(id) => id,
            None => {
                connection.execute(
                    "INSERT INTO playlists VALUES (NULL, :playlist_name, :user_id)",
                    named_params! { ":playlist_name": &name, ":user_id": &user_id.as_u64() },
                )?;

                connection.last_insert_rowid()
            }
        };

        // Insert songs
        let query = &format!(
            "INSERT OR IGNORE INTO songs VALUES {}",
            &songs
                .iter()
                .map(|s| format!(
                    "('{}', '{}')",
                    s.url,
                    &escape_json(serde_json::to_value(s.clone()).unwrap().to_string())
                ))
                .collect::<Vec<String>>()
                .join(", ")
        );

        connection.execute(query, ())?;

        connection.execute(
            format!(
                "INSERT OR REPLACE INTO playlists_map VALUES {}",
                &songs
                    .iter()
                    .map(|s| format!("(NULL, '{}', '{}')", &playlist_id, &s.url))
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .as_str(),
            (),
        )?;

        Ok(())
    }

    fn delete_playlist_song(
        &self,
        user_id: UserId,
        name: &String,
        url: &String,
    ) -> Result<(), DBError> {
        if self.is_disabled() {
            return Ok(());
        }

        let connection = self.get_connection()?;

        connection.execute("PRAGMA foreign_keys = ON", ())?;

        match connection.execute(
            "
                DELETE FROM playlists_map
                WHERE song_url=?1 AND 
                      id=(
                        SELECT id 
                        FROM playlists
                        WHERE name=?2 AND user_id=?3
                      );
                ",
            (url, name, user_id.as_u64()),
        ) {
            Ok(_) => (),
            Err(err) => {
                error!("[sqlite] {}", err.to_string());
                return Err(format!("Failed to delete song: {}", err).into());
            }
        }

        Ok(())
    }

    fn get_playlist(
        &self,
        user_id: UserId,
        name: &String,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        self._get_playlist(user_id, name, amount, offset, false)
    }

    fn set_history(&self, user_id: UserId, info: &MediaInfo) -> Result<(), DBError> {
        self.add_playlist_songs(user_id, &HISTORY_PLAYLIST.to_string(), vec![info])
    }

    fn get_history(
        &self,
        user_id: UserId,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<MediaInfo>, usize), DBError> {
        self._get_playlist(user_id, &HISTORY_PLAYLIST.to_string(), amount, offset, true)
    }

    fn get_playlists(
        &self,
        user_id: UserId,
        amount: usize,
        offset: usize,
    ) -> Result<(Vec<String>, usize), DBError> {
        let connection = self.get_connection()?;

        // Get playlists
        let mut statement = connection.prepare(&format!(
            "
            SELECT name
            FROM playlists
            WHERE playlists.user_id=:user_id AND playlists.name!='{HISTORY_PLAYLIST}'
            ORDER BY id DESC
            LIMIT :limit
            OFFSET :offset
            "
        ))?;

        let query = statement.query_map(
            named_params! { ":user_id": user_id.as_u64(), ":limit": amount, ":offset": offset },
            |r| r.get::<_, String>(0),
        )?;

        let playlists: Vec<String> = query.filter_map(|m| m.ok()).collect();

        // Get size
        let mut statement = connection.prepare(&format!(
            "
            SELECT count(*)
            FROM playlists
            WHERE playlists.user_id=?1 AND playlists.name!='{HISTORY_PLAYLIST}'
            "
        ))?;

        let size: i64 = statement
            .query(params![user_id.as_u64()])?
            .next()?
            .unwrap()
            .get(0)?;

        Ok((playlists, size as usize))
    }

    fn search_playlists(
        &self,
        user_id: UserId,
        search_term: &String,
    ) -> Result<Vec<String>, DBError> {
        let connection = self.get_connection()?;

        // Get playlists
        let mut statement = connection.prepare(&format!(
            "
            SELECT name
            FROM playlists
            WHERE playlists.user_id=:user_id 
                AND INSTR(lower(playlists.name), lower(:search)) > 0 
                AND playlists.name!='{HISTORY_PLAYLIST}'
            ORDER BY name ASC
            "
        ))?;

        let query = statement.query_map(
            named_params! { ":user_id": user_id.as_u64(), ":search": search_term },
            |r| r.get::<_, String>(0),
        )?;

        Ok(query.filter_map(|m| m.ok()).collect::<Vec<String>>())
    }
}

#[cfg(test)]
mod tests {
    use serial_test::serial;

    use super::*;

    const TEST_DB: &str = "test.sqlite";

    fn mock_db_plugin() -> SQLitePlugin {
        let connection = Connection::open_with_flags(
            TEST_DB,
            OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_FULL_MUTEX,
        )
        .unwrap();

        // Clear database
        connection
            .execute_batch(
                "
            PRAGMA writable_schema = 1;
            DELETE FROM sqlite_master;
            PRAGMA writable_schema = 0;
            VACUUM;
            PRAGMA integrity_check;
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

        assert!(plugin.set_history(UserId(1), &MediaInfo::empty()).is_ok());
    }

    #[test]
    #[serial]
    fn set_playlist_escape_single_quote() {
        let db = mock_db_plugin();
        let user_id = UserId(1);
        let playlist = "playlist".to_string();

        let song = MediaInfo {
            url: "test_url".to_string(),
            description: "It's".to_string(),
            duration: 0,
            title: "".to_string(),
            thumbnail: "".to_string(),
            uploader: "".to_string(),
            playlist: None,
        };

        let result = db.add_playlist_songs(user_id, &playlist, vec![&song]);

        assert!(result.is_ok());

        let playlist_songs = db.get_playlist(user_id, &playlist, 1, 0).unwrap().0;

        assert_eq!(playlist_songs[0].description, song.description);
    }

    #[test]
    #[serial]
    fn set_playlist_escape_tokens() {
        let db = mock_db_plugin();
        let user_id = UserId(1);
        let playlist = "playlist".to_string();

        let song = MediaInfo {
            url: "test_url".to_string(),
            description: "');".to_string(),
            duration: 0,
            title: "".to_string(),
            thumbnail: "".to_string(),
            uploader: "".to_string(),
            playlist: None,
        };

        let result = db.add_playlist_songs(user_id, &playlist, vec![&song]);

        assert!(result.is_ok());

        let playlist_songs = db.get_playlist(user_id, &playlist, 1, 0).unwrap().0;

        assert_eq!(playlist_songs[0].description, song.description);
    }

    #[test]
    #[serial]
    fn history_e2e() -> rusqlite::Result<()> {
        let db = mock_db_plugin();

        let user_id = UserId(1);

        let song = mock_info("url1");

        db.set_history(user_id, &song).unwrap();

        let connection = db.get_connection().unwrap();

        let mut statement = connection.prepare("SELECT * FROM users")?;
        let mut user_row = statement.query([])?;
        let user_row = user_row.next()?.unwrap();

        let mut statement = connection.prepare("SELECT * FROM songs")?;
        let mut song_row = statement.query([])?;
        let song_row = song_row.next()?.unwrap();

        let mut statement = connection.prepare("SELECT * FROM playlists")?;
        let mut playlist_row = statement.query([])?;
        let playlist_row = playlist_row.next()?.unwrap();

        let mut statement = connection.prepare("SELECT * FROM playlists_map")?;
        let mut playlist_map = statement.query([])?;
        let playlist_map = playlist_map.next()?.unwrap();

        assert_eq!(user_row.get::<_, i64>(0).unwrap(), *user_id.as_u64() as i64);
        assert_eq!(song_row.get::<_, String>(0).unwrap(), song.url);

        assert_eq!(playlist_row.get::<_, String>(1).unwrap(), HISTORY_PLAYLIST);

        assert_eq!(
            playlist_row.get::<_, i64>(2).unwrap(),
            *user_id.as_u64() as i64
        );

        assert_eq!(playlist_map.get::<_, String>(2).unwrap(), song.url);

        Ok(())
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

        let user_id = UserId(1);
        let song_1 = mock_info("url1");
        let song_2 = mock_info("url2");
        let song_3 = mock_info("url3");
        let song_4 = mock_info("url4");
        let song_5 = mock_info("url5");

        db.set_history(user_id, &song_1).unwrap();
        db.set_history(user_id, &song_2).unwrap();
        db.set_history(user_id, &song_3).unwrap();
        db.set_history(user_id, &song_4).unwrap();
        db.set_history(user_id, &song_5).unwrap();

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

        let user_id = UserId(1);
        let song_1 = mock_info("url1");
        let song_2 = mock_info("url2");
        let song_3 = mock_info("url3");
        let song_4 = mock_info("url4");
        let song_5 = mock_info("url5");

        db.set_history(user_id, &song_1).unwrap();
        db.set_history(user_id, &song_2).unwrap();
        db.set_history(user_id, &song_3).unwrap();
        db.set_history(user_id, &song_4).unwrap();
        db.set_history(user_id, &song_5).unwrap();

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

        let user_id = UserId(1);
        let playlist_name = "playlist".to_string();
        let song1 = mock_info("sussy_url");
        let song2 = mock_info("extra_song_1");
        let song3 = mock_info("extra_song_2");

        db.add_playlist_songs(user_id, &playlist_name, vec![&song1, &song2, &song3])
            .unwrap();

        let (songs, total) = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap();

        assert_eq!(total, 3);
        assert_eq!(songs[0], song1);

        db.delete_playlist_song(user_id, &playlist_name, &song1.url)
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 3, 0).unwrap().0;

        assert_eq!(songs.len(), 2);
    }

    #[test]
    #[serial]
    fn delete_playlist() -> rusqlite::Result<()> {
        let db = mock_db_plugin();

        let user_id = UserId(5);
        let playlist_name = "amogus twerking compilation".to_string();

        let song1 = mock_info("song_1");
        let song2 = mock_info("song_2");

        db.add_playlist_songs(user_id, &playlist_name, vec![&song1, &song2])
            .unwrap();

        let songs = db.get_playlist(user_id, &playlist_name, 10, 0).unwrap().0;

        assert_eq!(songs.len(), 2);

        db.delete_playlist(user_id, &playlist_name).unwrap();

        let connection = db.get_connection().unwrap();

        let mut statement = connection.prepare("SELECT COUNT(*) FROM playlists_map;")?;
        let mut row = statement.query([])?;

        assert_eq!(row.next()?.unwrap().get::<_, i64>(0).unwrap(), 0);

        Ok(())
    }

    #[test]
    #[serial]
    fn create_and_list_playlists() {
        let user_id = UserId(1);
        let db = mock_db_plugin();

        db.create_playlist(user_id, &"playlist_1".to_string()).ok();
        db.create_playlist(user_id, &"playlist_2".to_string()).ok();
        db.create_playlist(user_id, &"playlist_3".to_string()).ok();

        let playlists = db.get_playlists(user_id, 3, 0).unwrap();

        assert_eq!(playlists.0[0], "playlist_3");
        assert_eq!(playlists.0[1], "playlist_2");
        assert_eq!(playlists.0[2], "playlist_1");
        // Count
        assert_eq!(playlists.1, 3);
    }
}
