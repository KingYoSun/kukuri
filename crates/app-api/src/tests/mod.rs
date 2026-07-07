use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures_util::StreamExt;
use iroh::address_lookup::{AddrFilter, AddressLookup};
use iroh_mainline_address_lookup::DhtAddressLookup;
use kukuri_blob_service::IrohBlobService;
use kukuri_core::build_post_envelope_with_payload;
use kukuri_docs_sync::IrohDocsNode;
use kukuri_docs_sync::IrohDocsSync;
use kukuri_store::{
    BookmarkedCustomReactionRow, DirectMessageStore, LiveGameProjectionStore, MemoryStore,
    ReactionBookmarkStore, SocialProjectionStore, SqliteStore,
};
use kukuri_transport::{
    DhtDiscoveryOptions, DiscoveryMode, FakeNetwork, FakeTransport, HintEnvelope, HintStream,
    IrohGossipTransport, SeedPeer, TransportRelayConfig,
};
use n0_mainline::{DhtBuilder, Testnet};
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::sync::{Mutex as TokioMutex, broadcast};
use tokio::time::{Duration, sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;

mod capability_registry_snapshot;
mod direct_messages;
mod game;
mod live;
mod media;
mod notifications;
mod private_channels;
mod reactions;
mod social;
mod sync;
mod timeline;
mod topic_normalization;
mod views_wire_snapshot;

mod support;
use support::*;
