# mimicri

[![Version](https://img.shields.io/docker/v/poohcom1/mimicri?color=blue&logo=docker&style=flat-square)](https://hub.docker.com/repository/docker/poohcom1/mimicri)

A small, performant, easy to host Discord music bot written in Rust with multi-server history database functionality.

# Features

- [x] Search
  - [ ] Platform-specific/fallback search
- [x] Status (current song, queue, metadata)
- [x] Seeking
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
- sqlite3

### Config

Create a `.env` file in the root directory, and add the following variables:

```sh
DISCORD_TOKEN = *your bot token*
BOT_PREFIX = *bot prefix*

# Optional
SQLITE_DB = db.sqlite

# Debug
LOG_FILE = output.log
DEBUG_CHANNEL_ID = *channel id*
DEBUG_GUILD_ID = *guild id*
```

Alternatively, you can just set the environment variables before running.

| Variable           | Description                                                                |
| ------------------ | -------------------------------------------------------------------------- |
| `DISCORD_TOKEN`    | Required for bot to run                                                    |
| `BOT_PREFIX`       | Prefix for message commands                                                |
| `SQLITE_DB`        | Path to sqlite db file. If not present, playlist commands will be disabled |
| `LOG_FILE`         | Path to log file. If not present, log commands will be disabled            |
| `DEBUG_CHANNEL_ID` | Integer ID of channel to send startup message to for debugging             |
| `DEBUG_GUILD_ID`   | Integer ID of guild to manually register commands to for debugging         |

## Running

Make sure to set the required environment variables above.

For basic debugging, run:

```sh
cargo run
```

To create a release build:

```sh
cargo build --release
```

## Deploying

Use the `publish.py` python script to automatically update the version and push to remote. To do that, commit all your changes, and run the script below:

```sh
./publish.py bump -t patch #Type can be patch, minor, or major
```

The bump command does 3 things: bump the version on cargo, create a new commit containing only the bump, and tag the commit. With the `--push` flag, it will also automatically push to remote, triggering a workflow. You can run `./publish.py --help` to check out other tools and options.
