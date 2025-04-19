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
    if let Err(e) = client::run().await {
        tracing::error!(?e, "failed to run client");
        panic!("failed to run client: {e:?}");
    }
}
