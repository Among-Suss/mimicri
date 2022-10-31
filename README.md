# mimicri
*THIS PROJECT IS UNDER HEAVY DEVELOPMENT*

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
SQLITE_DB *database_filename.sqlite*
```

Alternatively, you can just set the environment variables before running.

## Running

Make sure to set the required environment variables.

For basic debugging, run:
```sh
cargo run
```

To create a release build:
```sh
cargo run --release
```

## Deploying

Use the `publish.py` python script to automatically update the version and push to remote. To do that, commit all your changes, and run the script below:

```sh
./publish.py bump --type patch --push #Type can be patch, minor, or major
```
The bump command does 3 things: bump the version on cargo, create a new commit containing only the bump, and tag the commit. With the `--push` flag, it will also automatically push to remote, triggering a workflow. You can run `./publish.py --help` to check out other tools and options.

*Note: If you're not using Unix Shell (a.k.a. you're on Windows), you might have to prefix the command with `py` or `python`.

