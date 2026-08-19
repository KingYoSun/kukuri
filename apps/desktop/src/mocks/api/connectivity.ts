import {
  type BlobMediaPayload,
  type CommunityNodeIndexQueryRequest,
  type DesktopApi,
  type IndexQueryResponse,
  type SubmitIndexingRequestResponse,
} from '@/lib/api';

import { cloneSyncStatus } from '../desktopMockModel';
import { type MockRuntime } from '../mockRuntime';

type ConnectivityMock = Pick<
  DesktopApi,
  | 'getSyncStatus'
  | 'getDiscoveryConfig'
  | 'getCommunityNodeConfig'
  | 'getCommunityNodeStatuses'
  | 'setCommunityNodeConfig'
  | 'clearCommunityNodeConfig'
  | 'authenticateCommunityNode'
  | 'setCommunityNodeInviteCode'
  | 'clearCommunityNodeToken'
  | 'getCommunityNodeConsentStatus'
  | 'acceptCommunityNodeConsents'
  | 'refreshCommunityNodeMetadata'
  | 'fetchCommunityNodeManifest'
  | 'readCommunityNodeTrustUser'
  | 'readCommunityNodeRelationUser'
  | 'listCommunityNodeRelationNeighbors'
  | 'getCommunityNodeRelationOptout'
  | 'setCommunityNodeRelationOptout'
  | 'clearCommunityNodeRelationOptout'
  | 'searchCommunityNodeIndex'
  | 'discoverCommunityNodeIndex'
  | 'recommendCommunityNodeIndex'
  | 'submitCommunityNodeIndexingRequest'
  | 'submitCommunityNodeReport'
  | 'importPeerTicket'
  | 'setDiscoverySeeds'
  | 'unsubscribeTopic'
  | 'setTopicGossipEnabled'
  | 'setChannelGossipEnabled'
  | 'getLocalPeerTicket'
  | 'getBlobMediaPayload'
  | 'getBlobPreviewUrl'
>;

export function createConnectivityMock(runtime: MockRuntime): ConnectivityMock {
  const {
    syncStatus,
    postsByTopic,
    liveSessionsByTopic,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    metaverseAssetPayloads,
    mockConsentItems,
  } = runtime;

  function queryIndex(request: CommunityNodeIndexQueryRequest): IndexQueryResponse {
    const query = request.query?.trim().toLocaleLowerCase() ?? '';
    const entries = Object.entries(postsByTopic)
      .flatMap(([topic, posts]) =>
        posts.map((post) => ({
          scope_kind: post.channel_id ? ('private_channel' as const) : ('public_topic' as const),
          scope_id: post.channel_id ?? topic,
          object_id: post.object_id,
          author_pubkey: post.author_pubkey,
          text: post.content,
          created_at: post.created_at,
        }))
      )
      .filter(
        (entry) =>
          (!request.scope_kind ||
            (entry.scope_kind === request.scope_kind && entry.scope_id === request.scope_id)) &&
          (!query || entry.text.toLocaleLowerCase().includes(query))
      )
      .sort((left, right) => right.created_at - left.created_at)
      .slice(0, request.limit ?? 20);
    return { entries };
  }

  const relationOptoutNodes = new Set<string>();

  return {
    async getSyncStatus() {
      return cloneSyncStatus(syncStatus);
    },
    async getDiscoveryConfig() {
      return runtime.discoveryConfig;
    },
    async getCommunityNodeConfig() {
      return runtime.communityNodeConfig;
    },
    async getCommunityNodeStatuses() {
      return runtime.communityNodeStatuses;
    },
    async setCommunityNodeConfig(nodes) {
      runtime.communityNodeConfig = {
        nodes: nodes.map((node) => ({
          base_url: node.base_url,
          auto_approve: node.auto_approve,
          resolved_urls: null,
        })),
      };
      runtime.communityNodeStatuses = nodes.map((node) => ({
        base_url: node.base_url,
        auto_approve: node.auto_approve,
        auth_state: { authenticated: false, expires_at: null },
        consent_state: null,
        resolved_urls: null,
        last_error: null,
        invite_code_saved: false,
        admission_rejection: null,
        session_phase: node.auto_approve ? 'connecting' : 'idle',
        retry_after: null,
        restart_required: false,
      }));
      return runtime.communityNodeConfig;
    },
    async clearCommunityNodeConfig() {
      runtime.communityNodeConfig = { nodes: [] };
      runtime.communityNodeStatuses = [];
    },
    async authenticateCommunityNode(baseUrl) {
      runtime.communityNodeStatuses = runtime.communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              auth_state: { authenticated: true, expires_at: Date.now() },
              consent_state: { all_required_accepted: false, items: mockConsentItems(false) },
              session_phase: status.auto_approve ? 'accepting' : 'authenticating',
            }
          : status
      );
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async setCommunityNodeInviteCode(baseUrl, inviteCode) {
      runtime.communityNodeStatuses = runtime.communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              invite_code_saved: Boolean(inviteCode?.trim()),
              admission_rejection: null,
              auth_state: { authenticated: true, expires_at: Date.now() },
              session_phase: 'authenticating',
            }
          : status
      );
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async clearCommunityNodeToken(baseUrl) {
      runtime.communityNodeStatuses = runtime.communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              auth_state: { authenticated: false, expires_at: null },
              consent_state: null,
              session_phase: 'idle',
            }
          : status
      );
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async getCommunityNodeConsentStatus(baseUrl) {
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async acceptCommunityNodeConsents(baseUrl) {
      const resolvedUrls = { public_base_url: baseUrl, connectivity_urls: [baseUrl] };
      syncStatus.discovery.connect_mode = 'direct_or_relay';
      runtime.communityNodeStatuses = runtime.communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              consent_state: { all_required_accepted: true, items: mockConsentItems(true) },
              resolved_urls: resolvedUrls,
              session_phase: 'ready',
              retry_after: null,
              restart_required: false,
            }
          : status
      );
      runtime.communityNodeConfig = {
        nodes: runtime.communityNodeConfig.nodes.map((node) =>
          node.base_url === baseUrl ? { ...node, resolved_urls: resolvedUrls } : node
        ),
      };
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async refreshCommunityNodeMetadata(baseUrl) {
      syncStatus.discovery.connect_mode = 'direct_or_relay';
      const resolvedUrls = { public_base_url: baseUrl, connectivity_urls: [baseUrl] };
      runtime.communityNodeStatuses = runtime.communityNodeStatuses.map((status) =>
        status.base_url === baseUrl
          ? {
              ...status,
              resolved_urls: resolvedUrls,
              session_phase: 'ready',
              retry_after: null,
              restart_required: false,
            }
          : status
      );
      runtime.communityNodeConfig = {
        nodes: runtime.communityNodeConfig.nodes.map((node) =>
          node.base_url === baseUrl ? { ...node, resolved_urls: resolvedUrls } : node
        ),
      };
      return runtime.communityNodeStatuses.find((status) => status.base_url === baseUrl)!;
    },
    async fetchCommunityNodeManifest(baseUrl) {
      return {
        status: 'ok',
        manifest: {
          node_id: '',
          node_name: baseUrl,
          node_role: 'community-node',
          server_name: baseUrl,
          manifest_version: 'v1',
          capability_scope: {
            // この mock は trust / relation 読み取りも提供するため、公開ノード情報でも
            // community_local_trust を提供中として宣言する(#705 の適格判定と一致させる)。
            available_enabled: [
              'auth_consent',
              'bootstrap_assist',
              'iroh_relay',
              'community_index',
              'community_local_trust',
            ],
            planned_enabled: ['moderation'],
          },
          authority_scope: {
            applies_to: ['this_node'],
            does_not_apply_to: [
              'kukuri_network_as_a_whole',
              'user_identity',
              'user_profile_canonical_source',
              'user_social_graph_canonical_source',
            ],
          },
          p2p_boundary: {
            identity_authority: false,
            profile_canonical_store: false,
            social_graph_canonical_store: false,
            content_truth_source: false,
            network_wide_authority: false,
          },
          abuse_contact: `abuse@${baseUrl.replace(/^https?:\/\//, '')}`,
          report_endpoint: `${baseUrl}/v1/report`,
          terms_url: `${baseUrl}/terms`,
          privacy_url: `${baseUrl}/privacy`,
          moderation_policy_url: `${baseUrl}/moderation-policy`,
        },
      };
    },
    async readCommunityNodeTrustUser(request) {
      return {
        viewer_pubkey: 'mock-viewer',
        target_id: request.target_pubkey,
        absolute: 0,
        relative: 0,
        trust: 0,
        w_abs_applied: 0.5,
        computed_at: new Date(0).toISOString(),
        basis: [],
      };
    },
    async readCommunityNodeRelationUser(request) {
      return {
        viewer_pubkey: 'mock-viewer',
        target_pubkey: request.target_pubkey,
        score: 0.5,
        basis: [
          {
            feature: 'shared_topics',
            value: 1,
            weight: 1,
            contribution: 0.5,
          },
        ],
      };
    },
    async listCommunityNodeRelationNeighbors() {
      const neighbors = Array.from(
        new Set(Object.values(postsByTopic).flatMap((posts) => posts.map((post) => post.author_pubkey)))
      );
      return { viewer_pubkey: 'mock-viewer', neighbors };
    },
    async getCommunityNodeRelationOptout(baseUrl) {
      return {
        pubkey: 'mock-viewer',
        opted_out: relationOptoutNodes.has(baseUrl),
        opted_out_at: relationOptoutNodes.has(baseUrl) ? new Date(0).toISOString() : null,
        min_proximity: 0.25,
      };
    },
    async setCommunityNodeRelationOptout(baseUrl) {
      relationOptoutNodes.add(baseUrl);
      return {
        pubkey: 'mock-viewer',
        opted_out: true,
        opted_out_at: new Date(0).toISOString(),
        min_proximity: 0.25,
      };
    },
    async clearCommunityNodeRelationOptout(baseUrl) {
      relationOptoutNodes.delete(baseUrl);
      return {
        pubkey: 'mock-viewer',
        opted_out: false,
        opted_out_at: null,
        min_proximity: 0.25,
      };
    },
    async searchCommunityNodeIndex(request) {
      return queryIndex(request);
    },
    async discoverCommunityNodeIndex(request) {
      return queryIndex(request);
    },
    async recommendCommunityNodeIndex(request) {
      return queryIndex(request);
    },
    async submitCommunityNodeIndexingRequest(request) {
      return {
        request_id: `mock-indexing-${request.scope_kind}-${request.channel_id ?? request.topic_id}`,
        status: 'pending',
      } satisfies SubmitIndexingRequestResponse;
    },
    async submitCommunityNodeReport(request) {
      return {
        status: 'submitted',
        reference_id: `mock-${request.subject_kind}-${request.subject_id}`,
        disputed_risk_signal_id: request.appeal?.risk_signal_id ?? null,
      };
    },
    async importPeerTicket() {},
    async setDiscoverySeeds(seedEntries) {
      runtime.discoveryConfig = {
        ...runtime.discoveryConfig,
        seed_peers: seedEntries.map((entry) => {
          const [endpointId, addrHint] = entry.split('@', 2);
          return {
            endpoint_id: endpointId,
            addr_hint: addrHint ?? null,
          };
        }),
      };
      syncStatus.discovery.configured_seed_peer_ids = runtime.discoveryConfig.seed_peers.map(
        (peer) => peer.endpoint_id
      );
      return runtime.discoveryConfig;
    },
    async unsubscribeTopic(topic) {
      delete postsByTopic[topic];
      delete liveSessionsByTopic[topic];
      delete gameRoomsByTopic[topic];
      delete joinedChannelsByTopic[topic];
      syncStatus.subscribed_topics = syncStatus.subscribed_topics.filter((value) => value !== topic);
      syncStatus.topic_diagnostics = syncStatus.topic_diagnostics.filter(
        (value) => value.topic !== topic
      );
    },
    async setTopicGossipEnabled(topic, enabled) {
      syncStatus.gossip_disabled_topics = syncStatus.gossip_disabled_topics.filter(
        (value) => value !== topic
      );
      if (!enabled) {
        syncStatus.gossip_disabled_topics.push(topic);
      }
    },
    async setChannelGossipEnabled(topic, channelId, enabled) {
      const key = `${topic}::${channelId}`;
      syncStatus.gossip_disabled_channels = syncStatus.gossip_disabled_channels.filter(
        (value) => value !== key
      );
      if (!enabled) {
        syncStatus.gossip_disabled_channels.push(key);
      }
    },
    async getLocalPeerTicket() {
      return 'peer1@127.0.0.1:7777';
    },
    async getBlobMediaPayload(hash, mime): Promise<BlobMediaPayload | null> {
      if (metaverseAssetPayloads[hash]) {
        return metaverseAssetPayloads[hash];
      }
      return {
        bytes_base64: mime.startsWith('video/') ? 'ZmFrZS12aWRlbw==' : 'ZmFrZS1pbWFnZQ==',
        mime,
      };
    },
    async getBlobPreviewUrl() {
      return null;
    },
  };
}
