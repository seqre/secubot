# Secubot

I'm not a programmer until I write my own Discord bot, right? And what is the best programming language to do so, if not Rust?

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