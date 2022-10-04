# mimicri
THIS PROJECT IS UNDER HEAVY DEVELOPMENT

A small, performant, easy to host Discord music bot written in Rust with multi-server history database functionality.

# Features
- [x] Search
  - [ ] Platform-specific/fallback search
- [ ] Status (current song, queue, metadata)
- [ ] Seeking
- [ ] Shuffling
- [ ] Playlists
- [ ] Timestamps
  - [ ] Timestamp shuffling
- [ ] Roles
- [x] Song history
  - [ ] Custom playlists
  - [ ] Randomized songs from history
- [ ] Voice Commands
- [ ] User-friend Interface

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

## Deploying

Use the `publish.py` python script to automatically update the version and push to remote. 

To increment a patch version:
```sh
./publish.py bump -t patch --push
```

To increment a minor version:
```sh
./publish.py bump -t minor --push
```

To increment a major version:
```sh
./publish.py bump -t major --push
```

*Note: If you're not using Unix Shell (a.k.a. you're on Windows), you might have to prefix the command with `py` or `python`.

You can run `./publish.py --help` to check out other tools and options.