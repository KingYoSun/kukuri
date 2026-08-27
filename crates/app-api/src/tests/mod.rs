use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use futures_util::StreamExt;
#[cfg(feature = "iroh-integration-tests")]
use iroh::address_lookup::{AddrFilter, AddressLookup};
#[cfg(feature = "iroh-integration-tests")]
use iroh_mainline_address_lookup::DhtAddressLookup;
#[cfg(feature = "iroh-integration-tests")]
use kukuri_blob_service::IrohBlobService;
use kukuri_core::build_post_envelope_with_payload;
#[cfg(feature = "iroh-integration-tests")]
use kukuri_docs_sync::IrohDocsSync;
#[cfg(feature = "iroh-integration-tests")]
use kukuri_iroh_node::IrohDocsNode;
use kukuri_store::{
    BookmarkedCustomReactionRow, ContentObservationRow, ContentObservationStore,
    DirectMessageStore, LiveGameProjectionStore, MemoryStore, ReactionBookmarkStore,
    SocialProjectionStore, SqliteStore,
};
#[cfg(feature = "iroh-integration-tests")]
use kukuri_transport::{DhtDiscoveryOptions, IrohGossipTransport, TransportRelayConfig};
use kukuri_transport::{
    DiscoveryMode, FakeNetwork, FakeTransport, HintEnvelope, HintStream, SeedPeer,
};
#[cfg(feature = "iroh-integration-tests")]
use n0_mainline::{DhtBuilder, Testnet};
use std::sync::OnceLock;
use tempfile::tempdir;
use tokio::sync::{Mutex as TokioMutex, broadcast};
use tokio::time::{Duration, sleep, timeout};
use tokio_stream::wrappers::BroadcastStream;

mod capability_registry_snapshot;
mod direct_messages;
mod dome_connections;
mod dome_hosting;
mod dome_move;
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
