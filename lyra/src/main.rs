mod client;
mod command;
mod component;
mod core;
mod error;
mod gateway;
mod lavalink;
mod runner;

pub use {
    error::command::Error as CommandError,
    lavalink::{ClientAndGuildIdAware as LavalinkAndGuildIdAware, ClientAware as LavalinkAware},
};

#[tokio::main]
#[tracing::instrument]
async fn main() {
    let _ = client::run().await;
}
