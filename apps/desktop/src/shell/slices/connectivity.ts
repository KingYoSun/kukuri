import type {
  CommunityNodeConfig,
  CommunityNodeManifest,
  CommunityNodeNodeStatus,
  CommunityNodePolicyDocument,
  DiscoveryConfig,
  SyncStatus,
} from '@/lib/api';
import type { CommunityIndexNodePreference } from '@/lib/api/communityIndex';

/// 接続まわり(discovery / community node / sync 状態)(WP-H6 PR3 のドメインスライス)。

export type CommunityNodeDraftNode = {
  id: string;
  base_url: string;
};

// public manifest endpoint (#356) からの取得状態。base_url ごとに保持する。
export type CommunityNodeManifestEntry =
  | { status: 'loading' }
  | { status: 'ok'; manifest: CommunityNodeManifest }
  | { status: 'absent' }
  | { status: 'error'; error: string };

// #857: 認証不要の公開 policy カタログ(GET /v1/policies)の取得状態。base_url ごとに保持する。
export type CommunityNodePoliciesEntry =
  | { status: 'loading' }
  | { status: 'ok'; policies: CommunityNodePolicyDocument[] }
  | { status: 'error'; error: string };

export type ConnectivitySliceState = {
  peerTicket: string;
  localPeerTicket: string | null;
  discoveryConfig: DiscoveryConfig;
  discoverySeedInput: string;
  discoveryEditorDirty: boolean;
  discoveryError: string | null;
  communityNodeConfig: CommunityNodeConfig;
  communityNodeStatuses: CommunityNodeNodeStatus[];
  communityNodeManifests: Record<string, CommunityNodeManifestEntry>;
  communityNodePolicies: Record<string, CommunityNodePoliciesEntry>;
  communityNodeInput: CommunityNodeDraftNode[];
  communityNodeEditorDirty: boolean;
  communityNodeError: string | null;
  communityIndexNodeBaseUrl: string | null;
  communityIndexNodePreference: CommunityIndexNodePreference;
  syncStatus: SyncStatus;
};

export const DEFAULT_DISCOVERY_CONFIG: DiscoveryConfig = {
  mode: 'seeded_dht',
  connect_mode: 'direct_only',
  env_locked: false,
  seed_peers: [],
};

export const DEFAULT_COMMUNITY_NODE_CONFIG: CommunityNodeConfig = {
  nodes: [],
};

export const DEFAULT_SYNC_STATUS: SyncStatus = {
  connected: false,
  delivery_state: 'Offline',
  peer_count: 0,
  pending_events: 0,
  status_detail: '',
  last_error: null,
  configured_peers: [],
  subscribed_topics: [],
  active_path: 'direct_p2p',
  fallback_peer_ids: [],
  topic_diagnostics: [],
  local_author_pubkey: '',
  discovery: {
    mode: 'seeded_dht',
    connect_mode: 'direct_only',
    active_path: 'direct_p2p',
    fallback_peer_ids: [],
    env_locked: false,
    configured_seed_peer_ids: [],
    bootstrap_seed_peer_ids: [],
    manual_ticket_peer_ids: [],
    connected_peer_ids: [],
    docs_assist_peer_ids: [],
    blob_assist_peer_ids: [],
    local_endpoint_id: '',
    last_discovery_error: null,
  },
  gossip_disabled_topics: [],
  gossip_disabled_channels: [],
};

export function createInitialConnectivitySlice(): ConnectivitySliceState {
  return {
    peerTicket: '',
    localPeerTicket: null,
    discoveryConfig: DEFAULT_DISCOVERY_CONFIG,
    discoverySeedInput: '',
    discoveryEditorDirty: false,
    discoveryError: null,
    communityNodeConfig: DEFAULT_COMMUNITY_NODE_CONFIG,
    communityNodeStatuses: [],
    communityNodeManifests: {},
    communityNodePolicies: {},
    communityNodeInput: [],
    communityNodeEditorDirty: false,
    communityNodeError: null,
    communityIndexNodeBaseUrl: null,
    communityIndexNodePreference: { mode: 'auto' },
    syncStatus: DEFAULT_SYNC_STATUS,
  };
}
