use serenity::async_trait;
use serenity::http::{Http};
use serenity::model::prelude::{GuildId, Message};
use songbird::input::{Restartable, Input};
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

    pub request_msg_channel: serenity::model::prelude::ChannelId,
    pub request_msg_http: Arc<Http>,
}

pub struct MediaQueue {
    pub now_playing: Option<(MediaItem, TrackHandle)>,
    pub queue: LinkedList<MediaItem>,
}

pub struct ChannelMediaPlayer {
    pub guild_id: GuildId,
    pub lock_protected_media_queue: (async_std::sync::Mutex<MediaQueue>, async_std::sync::Condvar),
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

pub async fn media_player_skip(
    shared_channel_media_player: Arc<ChannelMediaPlayer>
){
    let (shared_media_queue_lock, _) = &shared_channel_media_player.lock_protected_media_queue;
    let smq_locked = shared_media_queue_lock.lock().await;
    match &smq_locked.now_playing {
        Some((_, track_handle)) => {
            let result = track_handle.stop();
            match result {
                Ok(_) => (),
                Err(x) => {
                    println!("Error skipping track: {:?}", x);
                },
            }
        },
        None => (),
    };
}

pub async fn media_player_enqueue(
    url: String, 
    request_msg_channel: serenity::model::prelude::ChannelId,
    request_msg_http: Arc<Http>,
    shared_channel_media_player: Arc<ChannelMediaPlayer>
){
    let (shared_media_queue_lock, shared_media_queue_condvar) = &shared_channel_media_player.lock_protected_media_queue;

    let mut smq_locked = shared_media_queue_lock.lock().await;

    smq_locked.queue.push_front(MediaItem {
        url: url,
        title: String::new(),
        duration: 0,
        description: String::new(),
        metadata: HashMap::new(),
        request_msg_channel,
        request_msg_http,
    });

    shared_media_queue_condvar.notify_one();
}

pub async fn media_player_run(
    voice_channel_handler: Arc<serenity::prelude::Mutex<Call>>, 
    shared_channel_media_player: Arc<ChannelMediaPlayer>
){

    let (shared_media_queue_lock, shared_media_queue_condvar) = &shared_channel_media_player.lock_protected_media_queue;

    'medialoop: loop {

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
            let request_msg_channel = next_song.request_msg_channel;
            let request_msg_http = next_song.request_msg_http.clone();
            let source = match Restartable::ytdl(next_song.url.clone(), false).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Error creating source: {:?}", why);

                    check_msg(request_msg_channel.say(&request_msg_http, "Error playing track: youtube-dl or ffmpeg failed").await);
    
                    continue 'medialoop;
                }
            };
            let (track, track_handle) = songbird::create_player(Input::from(source));
            shared_media_queue.now_playing = Some((next_song, track_handle.clone()));
            
            // create a condvar to signal the end of the song
            // give the condvar to media event handler
            // register thee handler
            let media_event_handler :MediaEventHandler = MediaEventHandler::new(end_signaler.clone());
            match track_handle.add_event(songbird::Event::Track(songbird::TrackEvent::End), media_event_handler) {
                Ok(_) => (),
                Err(err) => {
                    println!("Error on track_handle.add_event {:?}", err);

                    check_msg(request_msg_channel.say(&request_msg_http, "Error playing track: Unable to initialize TrackEvent handler.").await);

                    continue 'medialoop;
                },
            }

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


/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: Result<Message, serenity::Error>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}
