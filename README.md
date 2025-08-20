<p align="center">
  <img alt="Λύρα banner" src="assets/lyra-banner.png">
</p>

# Λύρα

[![Built with Nix](https://img.shields.io/static/v1?logo=nixos&logoColor=white&label=&message=Built%20with%20Nix&color=41439a)](https://builtwithnix.org)
[![Community Chat](https://discordapp.com/api/guilds/1033430025527103568/widget.png?style=shield)](https://discord.gg/d4UerJpvTp)
[![Code Quality](https://www.codefactor.io/repository/github/lyra-music/lyra/badge)](https://www.codefactor.io/repository/github/lyra-music/lyra)
[![Build Status](https://github.com/lyra-music/lyra/actions/workflows/ci.yml/badge.svg?branch=main&event=push)](https://github.com/lyra-music/lyra/actions/workflows/ci.yml)
[![Latest Release](https://img.shields.io/github/release/lyra-music/lyra.svg)](https://github.com/lyra-music/lyra/releases/latest)
![License](https://img.shields.io/github/license/lyra-music/lyra)

A *self-hostable* **Discord music bot**, focused on *fairness*.

> [!NOTE]
> Λύρα is actively developed with a strong focus on maintaining stability.
> Core functionality is fully implemented, but some features (e.g., queue visualisation, command fairness polls) are still under development.
> Users are encouraged to self-host and test locally.

---

## Setup

Start by creating your environment configuration:

```console
$ cp .env.example .env
```

Edit `.env` and provide the required values:
- `BOT_TOKEN` (Discord bot token)
- Database credentials
- Lavalink credentials

Comments in the file will guide you through the configuration.

---

### Running

The easiest way to run Λύρα is via Docker. Begin by copying the example Compose file:

```console
$ cp compose.example.yaml compose.yaml
```

Next, set up two directories for persistent storage (PostgreSQL data and Lavalink plugins) and update your `.env`:

```console
$ mkdir -p /path/to/your/database
$ mkdir -p /path/to/your/plugins
# chown -R 322:322 /path/to/your/plugins
```

Update `.env`:

```dotenv
DOCKER_POSTGRES_PATH=/path/to/your/database
DOCKER_LAVALINK_PLUGINS_PATH=/path/to/your/plugins
```

Start the bot and services:

```console
# docker compose up -d
```

- View logs:
  ```console
  # docker compose logs -f
  ```
- Stop services:
  ```console
  # docker compose down
  ```

---

### Development

If you're using [`nix-direnv`](https://github.com/nix-community/nix-direnv), enter the dev shell automatically; otherwise, run:

```console
$ nix develop --no-pure-eval
```

This sets up all dependencies. Then, start the required services:

```console
$ devenv up -D
```

- Attach to logs:
  ```console
  $ process-compose attach
  ```
- Stop services:
  ```console
  $ process-compose down
  ```
- Run the bot:
  ```console
  $ cargo run --release
  ```

---

### Manual Setup (Not Recommended)

To set up manually, install:

- [Rust](https://www.rust-lang.org/tools/install)
- [PostgreSQL](https://www.postgresql.org/download/)
- [Lavalink](https://lavalink.dev/getting-started/index.html)

Follow their official documentation for configuration. Then:

```console
$ cargo run --release
```

---

## Attributions

- [twilight-rs](https://twilight.rs/) - Scalable Rust libraries for Discord API.
- [Lavalink](https://lavalink.dev/) - Standalone audio node based on Lavaplayer.
  - [lavalink-rs](https://gitlab.com/vicky5124/lavalink-rs/) - Async Lavalink bindings for Rust Discord libraries.
