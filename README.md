# mimicri
THIS PROJECT IS UNDER HEAVY DEVELOPMENT

A small, performant, easy to host Discord music bot written in Rust with multi-server history database functionality.

# Features
- [ ] Search
  - [ ] Platform-specific/fallback search
- [ ] Status (current song, queue, metadata)
- [ ] Seeking
- [ ] Shuffling
- [ ] Playlists
- [ ] Timestamps
  - [ ] Timestamp shuffling
- [ ] Roles
- [ ] Song history
  - [ ] Custom playlists
  - [ ] Randomized songs from history

## Setup

### Dependencies
 - youtube-dl
 - ffmpeg
 
### Optional Dependencies
 - sqlite

### Config
Create a `.env` file in the root directory, and add the following variables:
```sh
DISCORD_TOKEN = *your bot token*
BOT_PREFIX = *bot prefix*

# Optional
PLUGIN_SQLITE= *database_filename.sqlite*
```

Alternatively, you can just set the environment variables before running.

## Running

Make sure to set the required environment variables.

For basic debugging, run:
```sh
cargo run
```

For deploying:
```sh
cargo run --release
```
