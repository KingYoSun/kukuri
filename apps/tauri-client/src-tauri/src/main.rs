// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::Result;
use iroh::{Endpoint, SecretKey, protocol::Router};
use iroh_gossip::{net::Gossip, proto::TopicId};

#[tokio::main]
async fn main() -> Result<()> {
    let secret_key = SecretKey::generate(rand::rngs::OsRng);
    let endpoint = Endpoint::builder()
        .secret_key(secret_key)
        // Enable n0 discovery. This allows you to dials by 'NodeId', and allows you to be dialed by 'NodeId'.
        .discovery_n0()
        .bind()
        .await?;

    println!("> our node id: {}", endpoint.node_id());

    let gossip = Gossip::builder().spawn(endpoint.clone()).await?;

    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN, gossip.clone())
        .spawn().await?;

    let id = TopicId::from_bytes(rand::random());
    let node_ids = vec![];

    let topic = gossip.subscribe(id, node_ids)?;
    let (sender, _receiver) = topic.split();

    sender.broadcast("sup".into()).await?;

    router.shutdown().await?;

    tauri_client_lib::run();

    Ok(())
}
