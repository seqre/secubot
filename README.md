# Secubot
[![Rust CI](https://github.com/seqre/secubot/actions/workflows/rust.yml/badge.svg)](https://github.com/seqre/secubot/actions/workflows/rust.yml)

I'm not a programmer until I write my own Discord bot, right? And what is the best programming language to do so, if not Rust?

The bot was written at first out of the [Cyber Warriors](https://techcyberwarriors.org/)' need for ping cannon replacement, and over time I just kept adding new features.

## Features
- The Mighty Ping Cannon - pings provided users for 10 minutes after which it times out, allows for adding and removing users while pinging
- TODO lists - provides per-channel TODO lists backed by database, allows to specify assignee, it also posts periodical reminders about uncompleted todos
- Bot versioning - allows for checking the latest release notes and see currently running version

## Running

### Compile locally
TODO

### Docker
Build the image locally with `docker build .` or pull `ghcr.io/seqre/secubot` image.

Use the following command to quickly run the bot to test it:
```shell
docker run -e SCBT__DISCORD_TOKEN="token" secubot:latest
```

To have local persistent SQLite database, run:
```shell
touch db.sqlite                     // we need to create file first as docker cannot mount non-existing file
docker run \
  -v ${PWD}/db.sqlite:/db.sqlite \  // mount `db.sqlite` to have persistent database
  -e SCBT__DISCORD_TOKEN="token" \  // provide Discord token
  --name secubot \
  secubot:latest
```

To use proper config file(s), add the following mounts:
```shell
touch db.sqlite
docker run \
  -v ${PWD}/config.yaml:/config.yaml:ro \ // mount `config.yaml` file as singular configuration file
  -v ${PWD}/config:/config:ro \           // mount whole `config/` directory if you need to have multiple configuration files
  -v ${PWD}/db.sqlite:/db.sqlite \        // mount `db.sqlite` to have persistent database
  --name secubot \
  secubot:latest
```
You can use any format that [config-rs](https://github.com/mehcode/config-rs) supports, YAML is given as an example.

## Configuration

TODO
