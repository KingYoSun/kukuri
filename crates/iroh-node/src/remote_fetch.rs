//! docs-sync / blob-service 共通のリモートフェッチ本体(WP-B14)。
//!
//! 「local miss → cooldown ゲート → peers 走査 → connect(5s)→ fetch(15s)→
//! ローカル再読込」のループは両 crate で同一アルゴリズム(どちらも
//! `iroh_blobs::ALPN` で connect し `blobs().remote().fetch` で取得)だったため、
//! ここに一本化した。呼び出し側に残るのはローカル取得と policy 判定のみ。
//! ピア台帳・リトライ状態そのものは kukuri-transport の共通実装(WP-H2)。

use std::fmt::Display;
use std::time::Duration;

use anyhow::Result;
use kukuri_transport::{PeerAddrBook, RemoteFetchRetryState, RemoteFetchStart};
use tokio::sync::Mutex;
use tokio::time::{Instant, timeout};
use tracing::{info, warn};

use crate::IrohDocsNode;

pub const REMOTE_FETCH_CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
pub const REMOTE_FETCH_TRANSFER_TIMEOUT: Duration = Duration::from_secs(15);

/// remote fetch の取得モード。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FetchMode {
    /// ローカルストアへ取り込んでから読む(docs-sync / blob-service の恒久取得)。
    Store,
    /// ストアを経由せず memory に直接取得する(safety scan 用の一時 fetch。#609)。
    Ephemeral,
}

/// local miss 後のリモートフェッチ一式(cooldown ゲート込み)。
///
/// `subject` はログ上の対象名("docs entry" / "blob")。`local_error` は
/// 呼び出し側のローカル取得が返したエラーで、ログにのみ使う。
/// 成功時はローカルストアへ取り込んだうえで bytes を返す。
pub async fn fetch_bytes_with_cooldown(
    node: &IrohDocsNode,
    peers: &PeerAddrBook,
    retries: &Mutex<RemoteFetchRetryState>,
    subject: &str,
    hash_text: &str,
    hash: iroh_blobs::Hash,
    local_error: impl Display,
) -> Result<Option<Vec<u8>>> {
    fetch_bytes_with_cooldown_mode(
        node,
        peers,
        retries,
        subject,
        hash_text,
        hash,
        local_error,
        FetchMode::Store,
    )
    .await
}

/// `fetch_bytes_with_cooldown` の一時取得版: 取得した bytes をローカルストアへ**書き込まない**。
///
/// safety scan の一時 fetch(#609)用。community node の no-permanent-blob-storage 前提を
/// 構造的に守る(スキャン後の破棄処理が不要になる)。
pub async fn fetch_bytes_ephemeral_with_cooldown(
    node: &IrohDocsNode,
    peers: &PeerAddrBook,
    retries: &Mutex<RemoteFetchRetryState>,
    subject: &str,
    hash_text: &str,
    hash: iroh_blobs::Hash,
    local_error: impl Display,
) -> Result<Option<Vec<u8>>> {
    fetch_bytes_with_cooldown_mode(
        node,
        peers,
        retries,
        subject,
        hash_text,
        hash,
        local_error,
        FetchMode::Ephemeral,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn fetch_bytes_with_cooldown_mode(
    node: &IrohDocsNode,
    peers: &PeerAddrBook,
    retries: &Mutex<RemoteFetchRetryState>,
    subject: &str,
    hash_text: &str,
    hash: iroh_blobs::Hash,
    local_error: impl Display,
    mode: FetchMode,
) -> Result<Option<Vec<u8>>> {
    match retries.lock().await.try_begin(hash_text, Instant::now()) {
        RemoteFetchStart::Ready => {}
        RemoteFetchStart::CoolingDown => {
            info!(
                subject,
                hash = %hash_text,
                error = %local_error,
                "remote fetch skipped during retry cooldown"
            );
            return Ok(None);
        }
    }
    let result =
        fetch_bytes_from_remote(node, peers, subject, hash_text, hash, local_error, mode).await;
    retries
        .lock()
        .await
        .finish(hash_text, matches!(&result, Ok(Some(_))), Instant::now());
    result
}

async fn fetch_bytes_from_remote(
    node: &IrohDocsNode,
    peers: &PeerAddrBook,
    subject: &str,
    hash_text: &str,
    hash: iroh_blobs::Hash,
    local_error: impl Display,
    mode: FetchMode,
) -> Result<Option<Vec<u8>>> {
    let imported_peers = peers.merged_peers().await;
    info!(
        subject,
        hash = %hash_text,
        error = %local_error,
        configured_peer_count = imported_peers.len(),
        "fetch local miss, trying remote peers"
    );
    for imported_peer in imported_peers {
        let candidates = peers.connect_candidates(&imported_peer).await;
        info!(
            subject,
            hash = %hash_text,
            peer_id = %imported_peer.id,
            imported_addrs = ?imported_peer.addrs,
            candidate_count = candidates.len(),
            "fetch prepared remote peer candidates"
        );
        for peer in candidates {
            match timeout(
                REMOTE_FETCH_CONNECT_TIMEOUT,
                node.endpoint().connect(peer.clone(), iroh_blobs::ALPN),
            )
            .await
            {
                Ok(Ok(conn)) => {
                    info!(
                        subject,
                        hash = %hash_text,
                        peer_id = %peer.id,
                        addrs = ?peer.addrs,
                        "fetch connected to remote peer"
                    );
                    match mode {
                        FetchMode::Store => {
                            match timeout(
                                REMOTE_FETCH_TRANSFER_TIMEOUT,
                                node.blobs().remote().fetch(conn, hash),
                            )
                            .await
                            {
                                Ok(Ok(_)) => {
                                    info!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        "fetch remote transfer completed"
                                    );
                                }
                                Ok(Err(error)) => {
                                    warn!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        addrs = ?peer.addrs,
                                        error = %error,
                                        "fetch remote transfer failed"
                                    );
                                    continue;
                                }
                                Err(_) => {
                                    warn!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        addrs = ?peer.addrs,
                                        timeout_ms = REMOTE_FETCH_TRANSFER_TIMEOUT.as_millis(),
                                        "fetch remote transfer timed out"
                                    );
                                    continue;
                                }
                            }
                            match node.blobs().blobs().get_bytes(hash).await {
                                Ok(bytes) => return Ok(Some(bytes.to_vec())),
                                Err(error) => {
                                    warn!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        error = %error,
                                        "fetch transfer completed but content is still missing locally"
                                    );
                                }
                            }
                        }
                        FetchMode::Ephemeral => {
                            // ストアへ書き込まず、検証付きで memory へ直接取得する。
                            match timeout(
                                REMOTE_FETCH_TRANSFER_TIMEOUT,
                                iroh_blobs::get::request::get_blob(conn, hash).bytes(),
                            )
                            .await
                            {
                                Ok(Ok(bytes)) => {
                                    info!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        "ephemeral fetch remote transfer completed"
                                    );
                                    return Ok(Some(bytes.to_vec()));
                                }
                                Ok(Err(error)) => {
                                    warn!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        addrs = ?peer.addrs,
                                        error = %error,
                                        "ephemeral fetch remote transfer failed"
                                    );
                                    continue;
                                }
                                Err(_) => {
                                    warn!(
                                        subject,
                                        hash = %hash_text,
                                        peer_id = %peer.id,
                                        addrs = ?peer.addrs,
                                        timeout_ms = REMOTE_FETCH_TRANSFER_TIMEOUT.as_millis(),
                                        "ephemeral fetch remote transfer timed out"
                                    );
                                    continue;
                                }
                            }
                        }
                    }
                }
                Ok(Err(error)) => {
                    warn!(
                        subject,
                        hash = %hash_text,
                        peer_id = %peer.id,
                        addrs = ?peer.addrs,
                        error = %error,
                        "fetch connect failed"
                    );
                }
                Err(_) => {
                    warn!(
                        subject,
                        hash = %hash_text,
                        peer_id = %peer.id,
                        addrs = ?peer.addrs,
                        timeout_ms = REMOTE_FETCH_CONNECT_TIMEOUT.as_millis(),
                        "fetch connect timed out"
                    );
                }
            }
        }
    }
    warn!(
        subject,
        hash = %hash_text,
        "fetch exhausted remote peers without success"
    );
    Ok(None)
}
