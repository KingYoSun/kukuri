//! docs-sync / blob-service で共有するピア台帳とリモートフェッチのリトライ状態(WP-H2)。
//!
//! かつて両 crate にほぼ写しで存在したピア管理(learned / seed / imported の 3 台帳、
//! 合成順序、接続候補の優先順位、失敗クールダウン)の単一実装。
//!
//! **挙動差分は呼び出し側に残す**(REFACTORING.md / WP-H2 プランの決定):
//! - `insert_learned_peer_addr` は「台帳に変化があったか」の bool を返す。docs-sync は
//!   これを見て全レプリカへ同期先を配り直す(reapply)。blob-service は無視する。
//! - 状態復元(peer_state / restore)後の reapply も docs-sync 呼び出し側の責務。

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iroh::address_lookup::MemoryLookup;
use iroh::{Endpoint, EndpointAddr, EndpointId, RelayUrl};
use tokio::sync::Mutex;
// 元実装(docs-sync / blob-service)と同じ tokio の Instant を使う(テストでの時間制御と互換)。
use tokio::time::Instant;

use crate::config::SeedPeer;
use crate::tickets::relay_assisted_endpoint_addr;

pub const REMOTE_FETCH_RETRY_COOLDOWN: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RemoteFetchStart {
    Ready,
    CoolingDown,
}

/// リモートフェッチ失敗のクールダウン(対象キー毎)。
#[derive(Default)]
pub struct RemoteFetchRetryState {
    retry_after: BTreeMap<String, Instant>,
}

impl RemoteFetchRetryState {
    pub fn try_begin(&mut self, key: &str, now: Instant) -> RemoteFetchStart {
        if self
            .retry_after
            .get(key)
            .is_some_and(|retry_after| *retry_after > now)
        {
            return RemoteFetchStart::CoolingDown;
        }
        self.retry_after.remove(key);
        RemoteFetchStart::Ready
    }

    pub fn finish(&mut self, key: &str, success: bool, now: Instant) {
        if success {
            self.retry_after.remove(key);
        } else {
            self.retry_after
                .insert(key.to_string(), now + REMOTE_FETCH_RETRY_COOLDOWN);
        }
    }
}

/// direct(IP アドレス)だけを残した EndpointAddr(relay を含まない候補)。
pub fn direct_endpoint_addr(endpoint_addr: &EndpointAddr) -> Option<EndpointAddr> {
    let mut direct = EndpointAddr::new(endpoint_addr.id);
    for addr in endpoint_addr.ip_addrs() {
        direct = direct.with_ip_addr(*addr);
    }
    (!direct.is_empty()).then_some(direct)
}

/// learned / seed / imported の 3 台帳を持つピア台帳。
///
/// 合成順序(learned → seed → imported、id 重複排除)と接続候補の優先順位
/// (direct → remote_info 由来 → relay 付き → 元の値)は外部挙動として固定
/// (characterization: docs-sync / blob-service 双方の
/// `connect_candidates_prefers_direct_remote_info_before_relay_hint`)。
pub struct PeerAddrBook {
    endpoint: Endpoint,
    discovery: Arc<MemoryLookup>,
    learned_peers: Mutex<BTreeMap<String, EndpointAddr>>,
    seed_peers: Mutex<BTreeMap<String, EndpointAddr>>,
    imported_peers: Mutex<BTreeMap<String, EndpointAddr>>,
}

impl PeerAddrBook {
    pub fn new(endpoint: Endpoint, discovery: Arc<MemoryLookup>) -> Self {
        Self {
            endpoint,
            discovery,
            learned_peers: Mutex::new(BTreeMap::new()),
            seed_peers: Mutex::new(BTreeMap::new()),
            imported_peers: Mutex::new(BTreeMap::new()),
        }
    }

    /// 3 台帳の合成(learned → seed → imported の順、id 重複排除)。
    pub async fn merged_peers(&self) -> Vec<EndpointAddr> {
        let mut peers = self
            .learned_peers
            .lock()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for peer in self.seed_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        for peer in self.imported_peers.lock().await.values() {
            if !peers.iter().any(|existing| existing.id == peer.id) {
                peers.push(peer.clone());
            }
        }
        peers
    }

    /// learned 台帳へ挿入し、台帳に変化があったかを返す(同値なら false)。
    pub async fn insert_learned_peer_addr(&self, endpoint_addr: EndpointAddr) -> bool {
        if !endpoint_addr.is_empty() {
            self.discovery.add_endpoint_info(endpoint_addr.clone());
        }
        let key = endpoint_addr.id.to_string();
        let mut learned_peers = self.learned_peers.lock().await;
        if learned_peers.get(key.as_str()) == Some(&endpoint_addr) {
            return false;
        }
        learned_peers.insert(key, endpoint_addr);
        true
    }

    pub async fn insert_imported_peer_addr(&self, endpoint_addr: EndpointAddr) {
        self.discovery.add_endpoint_info(endpoint_addr.clone());
        self.imported_peers
            .lock()
            .await
            .insert(endpoint_addr.id.to_string(), endpoint_addr);
    }

    /// endpoint の remote_info と relay URL から learned ピアを記録し、
    /// 台帳に変化があったかを返す。
    pub async fn record_learned_peer(
        &self,
        endpoint_id: &str,
        relay_urls: &[RelayUrl],
    ) -> Result<bool> {
        let endpoint_id = EndpointId::from_str(endpoint_id.trim())?;
        let mut endpoint_addr = self
            .endpoint
            .remote_info(endpoint_id)
            .await
            .map(|remote_info| {
                EndpointAddr::from_parts(
                    remote_info.id(),
                    remote_info.into_addrs().map(|addr| addr.into_addr()),
                )
            })
            .unwrap_or_else(|| EndpointAddr::new(endpoint_id));
        for relay_url in relay_urls {
            endpoint_addr = endpoint_addr.with_relay_url(relay_url.clone());
        }
        Ok(self.insert_learned_peer_addr(endpoint_addr).await)
    }

    /// seed 台帳を丸ごと差し替える(SeedPeer → EndpointAddr 変換 + discovery 登録)。
    pub async fn set_seed_peers(
        &self,
        peers: Vec<SeedPeer>,
        relay_urls: &[RelayUrl],
    ) -> Result<()> {
        let mut parsed = BTreeMap::new();
        for peer in peers {
            let endpoint_addr = peer.to_endpoint_addr_with_relays(relay_urls)?;
            self.discovery.add_endpoint_info(endpoint_addr.clone());
            parsed.insert(endpoint_addr.id.to_string(), endpoint_addr);
        }
        *self.seed_peers.lock().await = parsed;
        Ok(())
    }

    /// 接続候補を優先順位つきで並べる: direct → remote_info 由来 → relay 付き → 元の値。
    pub async fn connect_candidates(&self, imported_peer: &EndpointAddr) -> Vec<EndpointAddr> {
        let mut candidates = Vec::new();
        if let Some(candidate) = direct_endpoint_addr(imported_peer) {
            candidates.push(candidate);
        }
        if let Some(remote_info) = self.endpoint.remote_info(imported_peer.id).await {
            let learned_peer = EndpointAddr::from_parts(
                remote_info.id(),
                remote_info.into_addrs().map(|addr| addr.into_addr()),
            );
            if !learned_peer.is_empty() {
                candidates.push(learned_peer);
            }
        }
        let relay_supported = relay_assisted_endpoint_addr(imported_peer);
        if relay_supported.relay_urls().next().is_some()
            && !candidates
                .iter()
                .any(|candidate| candidate == &relay_supported)
        {
            candidates.push(relay_supported);
        }
        if candidates.is_empty()
            || !candidates
                .iter()
                .any(|candidate| candidate == imported_peer)
        {
            candidates.push(imported_peer.clone());
        }
        candidates
    }

    /// 合成台帳のうち、endpoint がアクティブなアドレスを持つピア id を返す。
    pub async fn available_peer_ids(&self) -> Vec<String> {
        let peers = self.merged_peers().await;
        let mut available = BTreeSet::new();
        for peer in peers {
            if self
                .endpoint
                .remote_info(peer.id)
                .await
                .is_some_and(|info| {
                    info.addrs().any(|addr| {
                        matches!(addr.usage(), iroh::endpoint::TransportAddrUsage::Active)
                    })
                })
            {
                available.insert(peer.id.to_string());
            }
        }
        available.into_iter().collect()
    }

    /// 状態保存用のスナップショット(learned / imported。seed は設定から再構築される)。
    pub async fn learned_peers_snapshot(&self) -> Vec<EndpointAddr> {
        self.learned_peers.lock().await.values().cloned().collect()
    }

    pub async fn imported_peers_snapshot(&self) -> Vec<EndpointAddr> {
        self.imported_peers.lock().await.values().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // かつて docs-sync / blob-service に同名で重複していたテストの単一版(WP-H2)。
    #[test]
    fn remote_fetch_retry_state_cools_down_failures_without_blocking_active_fetches() {
        let now = Instant::now();
        let mut state = RemoteFetchRetryState::default();

        assert_eq!(state.try_begin("hash-a", now), RemoteFetchStart::Ready);
        assert_eq!(state.try_begin("hash-a", now), RemoteFetchStart::Ready);

        state.finish("hash-a", false, now);
        assert_eq!(
            state.try_begin("hash-a", now + Duration::from_secs(1)),
            RemoteFetchStart::CoolingDown
        );
        assert_eq!(
            state.try_begin("hash-a", now + REMOTE_FETCH_RETRY_COOLDOWN),
            RemoteFetchStart::Ready
        );

        state.finish("hash-a", true, now + REMOTE_FETCH_RETRY_COOLDOWN);
        assert_eq!(
            state.try_begin("hash-a", now + REMOTE_FETCH_RETRY_COOLDOWN),
            RemoteFetchStart::Ready
        );
    }
}
