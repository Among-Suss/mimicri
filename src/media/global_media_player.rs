use serenity::async_trait;
use serenity::model::prelude::GuildId;
use songbird::input::{Input, Restartable};
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, EventContext, EventHandler};
use std::collections::{HashMap, LinkedList};
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

use crate::message_context::MessageContext;

use super::media_info::MediaInfo;

struct MediaEventHandler {
    signaler: Arc<(async_std::sync::Mutex<bool>, async_std::sync::Condvar)>,
}

impl MediaEventHandler {
    fn new(signaler: Arc<(async_std::sync::Mutex<bool>, async_std::sync::Condvar)>) -> Self {
        MediaEventHandler { signaler }
    }
}

pub struct MediaItem {
    pub info: MediaInfo,
    pub message_ctx: MessageContext,
}

pub struct MediaQueue {
    pub running_state: bool,
    pub now_playing: Option<(MediaItem, TrackHandle)>,
    pub queue: LinkedList<Option<MediaItem>>,
}

pub struct ChannelMediaPlayer {
    pub guild_id: GuildId,
    pub lock_protected_media_queue: (async_std::sync::Mutex<MediaQueue>, async_std::sync::Condvar),
}

type GuildMediaPlayerMap = async_std::sync::Mutex<
    Option<HashMap<serenity::model::prelude::GuildId, Arc<ChannelMediaPlayer>>>,
>;

pub struct GlobalMediaPlayer {
    pub guild_media_player_map: GuildMediaPlayerMap,
}

impl GlobalMediaPlayer {
    pub const UNINITIALIZED: GlobalMediaPlayer = GlobalMediaPlayer {
        guild_media_player_map: async_std::sync::Mutex::new(None),
    };

    pub async fn init_self(&self) {
        let mut guild_map = self.guild_media_player_map.lock().await;
        match &*guild_map {
            Some(_) => panic!("HashMap should be uninitialized!"),
            None => *guild_map = Some(HashMap::new()),
        };
    }

    pub async fn start(
        &self,
        guild_id: GuildId,
        voice_channel_handler: Arc<serenity::prelude::Mutex<Call>>,
    ) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if guild_map.contains_key(&guild_id) {
            return Err(String::from(
                "Already connected to a voice channel in this server!",
            ));
        } else {
            guild_map.insert(
                guild_id,
                ChannelMediaPlayer::create_and_initialize(guild_id, voice_channel_handler),
            );
        }

        Ok(())
    }

    pub async fn skip(&self, guild_id: GuildId) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            media_player.skip().await;
            Ok(())
        } else {
            Err(String::from("Not connected to a voice channel!"))
        }
    }

    pub async fn quit(&self, guild_id: GuildId) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.remove(&guild_id) {
            media_player.quit().await;
            Ok(())
        } else {
            Err(String::from("Not connected to a voice channel!"))
        }
    }

    /// Reads the queue between start and length.
    ///
    /// Returns a tuple of the queue as a LinkedList and the total size of the queue
    pub async fn read_queue(
        &self,
        guild_id: GuildId,
        start: usize,
        length: usize,
    ) -> Result<(LinkedList<MediaInfo>, usize), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            Ok(media_player.read_queue(start, length).await)
        } else {
            Err(String::from("Not connected to a voice channel!"))
        }
    }

    pub async fn seek(&self, guild_id: GuildId, time: i64) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            media_player.seek(time).await
        } else {
            Err(String::from("Not connected to a voice channel!"))
        }
    }

    pub async fn now_playing(&self, guild_id: GuildId) -> Result<Option<(MediaInfo, i64)>, String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            media_player.now_playing().await
        } else {
            Err(String::from("Not connected to a voice channel!"))
        }
    }

    pub async fn enqueue(
        &self,
        guild_id: GuildId,
        info: MediaInfo,
        message_ctx: MessageContext,
    ) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            media_player.enqueue(info, message_ctx).await;
        } else {
            return Err("Not connected to a voice channel!".to_string());
        }

        Ok(())
    }

    pub async fn enqueue_batch(
        &self,
        guild_id: GuildId,
        infos: LinkedList<MediaInfo>,
        message_ctx: MessageContext,
    ) -> Result<(), String> {
        let mut guild_map_guard = self.guild_media_player_map.lock().await;
        let guild_map = guild_map_guard.as_mut().unwrap();

        if let Some(media_player) = guild_map.get(&guild_id) {
            media_player.enqueue_batch(infos, message_ctx).await;
        } else {
            return Err("Not connected to a voice channel!".to_string());
        }

        Ok(())
    }
}

#[async_trait]
impl EventHandler for MediaEventHandler {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let (mutex, condvar) = &*self.signaler;

        let mut guard = mutex.lock().await;
        *guard = true;
        condvar.notify_one();

        None
    }
}

impl ChannelMediaPlayer {
    fn create_and_initialize(
        guild_id: GuildId,
        voice_channel_handler: Arc<serenity::prelude::Mutex<Call>>,
    ) -> Arc<Self> {
        let media_player = Arc::new(ChannelMediaPlayer {
            guild_id,
            lock_protected_media_queue: (
                async_std::sync::Mutex::new(MediaQueue {
                    running_state: true,
                    now_playing: None,
                    queue: LinkedList::new(),
                }),
                async_std::sync::Condvar::new(),
            ),
        });

        tokio::spawn(Self::media_player_run(
            voice_channel_handler,
            media_player.clone(),
        ));

        media_player
    }

    async fn skip(&self) {
        let (shared_media_queue_lock, _) = &self.lock_protected_media_queue;
        let smq_locked = shared_media_queue_lock.lock().await;
        match &smq_locked.now_playing {
            Some((_, track_handle)) => {
                let result = track_handle.stop();
                match result {
                    Ok(_) => (),
                    Err(x) => {
                        error!("Error skipping track: {:?}", x);
                    }
                }
            }
            None => (),
        };
    }

    async fn seek(&self, time: i64) -> Result<(), String> {
        let (shared_media_queue_lock, _) = &self.lock_protected_media_queue;
        let smq_locked = shared_media_queue_lock.lock().await;

        match &smq_locked.now_playing {
            Some((media_item, track_handle)) => {
                if time < media_item.info.duration {
                    match track_handle.seek_time(Duration::from_secs(time as u64)) {
                        Ok(_) => Ok(()),
                        Err(err) => Err(format!("Unable to seek track: {:?}", err)),
                    }
                } else {
                    Err(format!("Unable to seek track: time out of range"))
                }
            }
            None => Err(String::from("Cannot seek, not playing a song.")),
        }
    }

    async fn now_playing(&self) -> Result<Option<(MediaInfo, i64)>, String> {
        let (shared_media_queue_lock, _) = &self.lock_protected_media_queue;
        let smq_locked = shared_media_queue_lock.lock().await;
        match &smq_locked.now_playing {
            Some((media_item, track_handle)) => {
                let result = track_handle.get_info().await;
                match result {
                    Ok(trackstate) => {
                        let position = trackstate.position.as_secs() as i64;
                        Ok(Some((media_item.info.clone(), position)))
                    }
                    Err(trackerror) => Err(format!(
                        "Unable to get current song info from Track: {}",
                        trackerror
                    )),
                }
            }
            None => Ok(None),
        }
    }

    async fn read_queue(&self, start: usize, length: usize) -> (LinkedList<MediaInfo>, usize) {
        let mut return_queue = LinkedList::new();

        let (shared_media_queue_lock, _) = &self.lock_protected_media_queue;

        let smq_locked = shared_media_queue_lock.lock().await;

        let (start, length) = if start == 0 {
            match &smq_locked.now_playing {
                Some((media_item, _)) => {
                    return_queue.push_front(media_item.info.clone());
                }
                None => return_queue.push_front(MediaInfo::empty()),
            }
            (start, length - 1)
        } else {
            (start - 1, length)
        };

        for (i, media_item) in smq_locked.queue.iter().rev().enumerate() {
            if i >= start + length {
                break;
            }

            if i >= start {
                match media_item {
                    Some(media_item) => return_queue.push_back(media_item.info.clone()),
                    None => return_queue.push_back(MediaInfo::empty()),
                }
            }
        }

        (
            return_queue,
            smq_locked.queue.len()
                + match smq_locked.now_playing {
                    Some(_) => 1,
                    None => 0,
                },
        )
    }

    async fn enqueue(&self, info: MediaInfo, message_ctx: MessageContext) {
        let (shared_media_queue_lock, shared_media_queue_condvar) =
            &self.lock_protected_media_queue;

        info!("Enqueuing song: {}", &info.title);

        let mut smq_locked = shared_media_queue_lock.lock().await;

        smq_locked
            .queue
            .push_front(Some(MediaItem { info, message_ctx }));

        shared_media_queue_condvar.notify_one();
    }

    async fn enqueue_batch(
        &self,
        mut media_infos: LinkedList<MediaInfo>,
        message_ctx: MessageContext,
    ) {
        let (shared_media_queue_lock, shared_media_queue_condvar) =
            &self.lock_protected_media_queue;

        let mut smq_locked = shared_media_queue_lock.lock().await;

        loop {
            let media_info = match media_infos.pop_front() {
                Some(x) => x,
                None => break,
            };
            smq_locked.queue.push_front(Some(MediaItem {
                info: media_info,
                message_ctx: message_ctx.clone(),
            }));
        }

        shared_media_queue_condvar.notify_one();
    }

    async fn quit(&self) {
        let (shared_media_queue_lock, _) = &self.lock_protected_media_queue;

        {
            let mut shared_media_queue = shared_media_queue_lock.lock().await;

            shared_media_queue.running_state = false;
            shared_media_queue.queue.push_front(None);

            match &shared_media_queue.now_playing {
                Some((_, track_handle)) => {
                    let res = track_handle.stop();
                    match res {
                        Ok(_) => (),
                        Err(e) => {
                            error!("unable to skip track to quit media player, {:?}", e);
                        }
                    }
                }
                None => (),
            }
        }
    }

    async fn media_player_run(
        voice_channel_handler: Arc<serenity::prelude::Mutex<Call>>,
        shared_channel_media_player: Arc<ChannelMediaPlayer>,
    ) {
        let (shared_media_queue_lock, shared_media_queue_condvar) =
            &shared_channel_media_player.lock_protected_media_queue;

        'medialoop: loop {
            let end_signaler = Arc::new((
                async_std::sync::Mutex::new(false),
                async_std::sync::Condvar::new(),
            ));

            let running_state = {
                // lock and wait for song queue to not be empty
                let mut shared_media_queue = shared_media_queue_lock.lock().await;
                while shared_media_queue.queue.is_empty() {
                    shared_media_queue = shared_media_queue_condvar.wait(shared_media_queue).await;
                }
                let next_song = shared_media_queue.queue.pop_back().unwrap();

                if !shared_media_queue.running_state || next_song.is_none() {
                    break 'medialoop;
                }

                // get song from queue and create source, track, trackhandle
                // set current song
                let next_song = next_song.unwrap();
                let message_ctx = next_song.message_ctx.clone();
                let source = match Restartable::ytdl(next_song.info.url.clone(), false).await {
                    Ok(source) => source,
                    Err(why) => {
                        error!("Error creating source: {:?}", why);

                        message_ctx
                            .send_error("Error playing track: youtube-dl or ffmpeg failed")
                            .await;

                        continue 'medialoop;
                    }
                };
                let (track, track_handle) = songbird::create_player(Input::from(source));
                shared_media_queue.now_playing = Some((next_song, track_handle.clone()));

                // create a condvar to signal the end of the song
                // give the condvar to media event handler
                // register thee handler
                let media_event_handler: MediaEventHandler =
                    MediaEventHandler::new(end_signaler.clone());
                match track_handle.add_event(
                    songbird::Event::Track(songbird::TrackEvent::End),
                    media_event_handler,
                ) {
                    Ok(_) => (),
                    Err(err) => {
                        error!("Error on track_handle.add_event {:?}", err);

                        message_ctx
                            .send_error(
                                "Error playing track: Unable to initialize TrackEvent handler.",
                            )
                            .await;

                        continue 'medialoop;
                    }
                }

                // play the track
                let mut vc_handler = voice_channel_handler.lock().await;
                vc_handler.play(track);

                shared_media_queue.running_state
            };

            // wait for song to finish
            if running_state {
                let (end_mutex, end_condvar) = &*end_signaler;
                let mut end_guard = end_mutex.lock().await;
                while !*end_guard {
                    end_guard = end_condvar.wait(end_guard).await;
                }
            }

            {
                let mut shared_media_queue = shared_media_queue_lock.lock().await;
                shared_media_queue.now_playing = None;
                if !shared_media_queue.running_state {
                    break 'medialoop;
                }
            }
        }

        {
            let mut shared_media_queue = shared_media_queue_lock.lock().await;
            shared_media_queue.now_playing = None;
        }
    }
}
