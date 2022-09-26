use serenity::async_trait;
use serenity::prelude::Mutex;
use serenity::model::prelude::GuildId;
use songbird::tracks::TrackHandle;
use songbird::{Call, EventHandler, EventContext, Event};
use std::collections::{LinkedList, HashMap};
use std::sync::Arc;

pub struct MediaEventHandler {
    signaler: Arc<(async_std::sync::Mutex<bool>, async_std::sync::Condvar)>,
}

impl MediaEventHandler {
    fn new(signaler: Arc<(async_std::sync::Mutex<bool>, async_std::sync::Condvar)>) -> Self {
        MediaEventHandler { signaler }
    }
}

pub struct MediaItem {
    pub url: String,
    pub title: String,
    pub duration: i64,
    pub description: String,
    pub metadata: HashMap<String, String>,
}

pub struct MediaQueue {
    pub now_playing: Option<(MediaItem, TrackHandle)>,
    pub queue: LinkedList<MediaItem>,
}

pub struct ChannelMediaPlayer {
    pub guild_id: GuildId,
    pub shared_media_queue: Arc<(Mutex<MediaQueue>, async_std::sync::Condvar)>,
}

#[async_trait]
impl EventHandler for MediaEventHandler {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        let (mutex, condvar) = &*self.signaler;

        let mut guard = mutex.lock().await;
        *guard = true;
        condvar.notify_one();

        None
    }
}

pub async fn run_media_player(
    voice_channel_handler: Arc<Mutex<Call>>, 
    shared_media_queue: Arc<(async_std::sync::Mutex<MediaQueue>, async_std::sync::Condvar)>
){

    let (shared_media_queue_lock, shared_media_queue_condvar) = &*shared_media_queue;

    loop {

        let end_signaler = Arc::new((async_std::sync::Mutex::new(false), async_std::sync::Condvar::new()));

        {

            // lock and wait for song queue to not be empty
            let mut shared_media_queue = shared_media_queue_lock.lock().await;
            while shared_media_queue.queue.is_empty() {
                shared_media_queue = shared_media_queue_condvar.wait(shared_media_queue).await;
            }

            // get song from queue and create source, track, trackhandle
            // set current song
            let next_song = shared_media_queue.queue.pop_back().unwrap();
            let source = match songbird::ytdl(&next_song.url).await {
                Ok(source) => source,
                Err(why) => {
                    panic!("ERROR; THIS SHOULD BE FIXED")
                }
            };
            let (track, track_handle) = songbird::create_player(source);
            shared_media_queue.now_playing = Some((next_song, track_handle.clone()));
            
            // create a condvar to signal the end of the song
            // give the condvar to media event handler
            // register thee handler
            let media_event_handler :MediaEventHandler = MediaEventHandler::new(end_signaler.clone());
            track_handle.add_event(songbird::Event::Track(songbird::TrackEvent::End), media_event_handler);

            // play the track
            let mut vc_handler = voice_channel_handler.lock().await;
            vc_handler.play(track);

        }
        
        // wait for song to finish
        let (end_mutex, end_condvar) = &*end_signaler;
        let mut end_guard = end_mutex.lock().await;
        while !*end_guard {
            end_guard = end_condvar.wait(end_guard).await;
        }

    }

}


