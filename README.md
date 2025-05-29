<div align="center">

# **Λύρα**

<img src="assets/lyra2-X.png"  width="256">

A *featureful*, *self-hostable* **Discord music bot**, made with [`twilight-rs`](https://twilight.rs/) and [`Lavalink`](https://github.com/freyacodes/Lavalink), written in [`Rust`](https://www.rust-lang.org/).

</div>

> [!WARNING]
> Still in early development!

## Setup

Start by copying the example environment file:

```console
$ cp .env.example .env
```

Then, edit the `.env` file to set your environment variables. The file contains comments to guide you. At a minimum you should provide a valid `BOT_TOKEN`, database credentials, and lavalink credentials.

### Docker

The easiest way to set up Λύρα is to use Docker. Start by creating a copy of the example docker compose file:

```console
$ cp compose.example.yaml compose.yaml
```

In addition, you need to set `DOCKER_POSTGRES_PATH` and `DOCKER_LAVALINK_PLUGINS_PATH` environment variables in `.env` to point to two empty directories you want to use for the database and plugins respectively. You can create them with:

```console
$ mkdir -p /path/to/your/database
$ mkdir -p /path/to/your/plugins
# chown -R 322:322 /path/to/your/plugins
```

```bash
# File: .env
DOCKER_POSTGRES_PATH=/path/to/your/database
DOCKER_LAVALINK_PLUGINS_PATH=/path/to/your/plugins
```

Then, run the following command to start the bot and the database:

```console
# docker compose up -d
```
This will start the bot and it's associated services in detached mode and run them in the background. To check the logs, run:

```console
# docker compose logs -f
```

To stop the bot, run:

```console
# docker compose down
```

### Nix

From the project root, run:

```console
$ nix develop --impure -c devenv up
```

This will download all the dependencies and set up the environment, as well as starting the services required for the bot to function.

Then, run the bot:

```console
$ nix develop --impure -c cargo run --release
```

### Manual (Not recommended)

If you want to set up the bot manually, you need to install the following dependencies:

- [`Rust`](https://www.rust-lang.org/tools/install)
- [`PostgreSQL`](https://www.postgresql.org/download/)
- [`Lavalink`](https://lavalink.dev/getting-started/index.html)

Follow the official documentation on how to set up and configure these tools.

Then, clone the repository and run the following command:

```console
$ cargo run --release
```
