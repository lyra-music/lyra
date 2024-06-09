mod client;
mod command;
mod component;
mod core;
mod error;
mod gateway;
mod lavalink;
mod runner;

#[tokio::main]
#[tracing::instrument]
async fn main() {
    let _ = client::run().await;
}
