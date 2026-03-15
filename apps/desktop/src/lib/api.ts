import { invoke } from '@tauri-apps/api/core';

export type TimelineCursor = {
  created_at: number;
  event_id: string;
};

export type PostView = {
  id: string;
  author_pubkey: string;
  author_npub: string;
  note_id: string;
  content: string;
  content_status: BlobViewStatus;
  attachments: AttachmentView[];
  created_at: number;
  reply_to?: string | null;
  root_id?: string | null;
};

export type BlobViewStatus = 'Missing' | 'Available' | 'Pinned';

export type AttachmentView = {
  hash: string;
  mime: string;
  bytes: number;
  role: string;
  status: BlobViewStatus;
};

export type CreateAttachmentInput = {
  file_name?: string | null;
  mime: string;
  byte_size: number;
  data_base64: string;
  role?: string | null;
};

export type BlobMediaPayload = {
  bytes_base64: string;
  mime: string;
};

export type TimelineView = {
  items: PostView[];
  next_cursor?: TimelineCursor | null;
};

export type DiscoveryMode = 'static_peer' | 'seeded_dht';

export type ConnectMode = 'direct_only' | 'direct_or_relay';

export type SeedPeer = {
  endpoint_id: string;
  addr_hint?: string | null;
};

export type DiscoveryConfig = {
  mode: DiscoveryMode;
  connect_mode: ConnectMode;
  env_locked: boolean;
  seed_peers: SeedPeer[];
};

export type DiscoveryStatus = {
  mode: DiscoveryMode;
  connect_mode: ConnectMode;
  env_locked: boolean;
  seed_peer_ids: string[];
  manual_ticket_peer_ids: string[];
  connected_peer_ids: string[];
  local_endpoint_id: string;
  last_discovery_error?: string | null;
};

export type SyncStatus = {
  connected: boolean;
  last_sync_ts?: number | null;
  peer_count: number;
  pending_events: number;
  status_detail: string;
  last_error?: string | null;
  configured_peers: string[];
  subscribed_topics: string[];
  topic_diagnostics: TopicSyncStatus[];
  local_author_pubkey: string;
  discovery: DiscoveryStatus;
};

export type CommunityNodeResolvedUrls = {
  public_base_url: string;
  relay_ws_url: string;
  iroh_relay_urls: string[];
};

export type CommunityNodeNodeConfig = {
  base_url: string;
  resolved_urls?: CommunityNodeResolvedUrls | null;
};

export type CommunityNodeConfig = {
  nodes: CommunityNodeNodeConfig[];
};

export type CommunityNodeAuthState = {
  authenticated: boolean;
  expires_at?: number | null;
};

export type CommunityNodeConsentItem = {
  policy_slug: string;
  policy_version: number;
  title: string;
  required: boolean;
  accepted_at?: number | null;
};

export type CommunityNodeConsentStatus = {
  all_required_accepted: boolean;
  items: CommunityNodeConsentItem[];
};

export type CommunityNodeNodeStatus = {
  base_url: string;
  auth_state: CommunityNodeAuthState;
  consent_state?: CommunityNodeConsentStatus | null;
  resolved_urls?: CommunityNodeResolvedUrls | null;
  last_error?: string | null;
  restart_required: boolean;
};

export type TopicSyncStatus = {
  topic: string;
  joined: boolean;
  peer_count: number;
  connected_peers: string[];
  configured_peer_ids: string[];
  missing_peer_ids: string[];
  last_received_at?: number | null;
  status_detail: string;
  last_error?: string | null;
};

export type LiveSessionStatus = 'Live' | 'Ended';

export type LiveSessionView = {
  session_id: string;
  host_pubkey: string;
  title: string;
  description: string;
  status: LiveSessionStatus;
  started_at: number;
  ended_at?: number | null;
  viewer_count: number;
  joined_by_me: boolean;
};

export type GameRoomStatus = 'Open' | 'InProgress' | 'Finished';

export type GameScoreView = {
  participant_id: string;
  label: string;
  score: number;
};

export type GameRoomView = {
  room_id: string;
  host_pubkey: string;
  title: string;
  description: string;
  status: GameRoomStatus;
  phase_label?: string | null;
  scores: GameScoreView[];
  updated_at: number;
};

export interface DesktopApi {
  createPost(
    topic: string,
    content: string,
    replyTo?: string | null,
    attachments?: CreateAttachmentInput[]
  ): Promise<string>;
  listTimeline(topic: string, cursor?: TimelineCursor | null, limit?: number): Promise<TimelineView>;
  listThread(
    topic: string,
    threadId: string,
    cursor?: TimelineCursor | null,
    limit?: number
  ): Promise<TimelineView>;
  listLiveSessions(topic: string): Promise<LiveSessionView[]>;
  createLiveSession(topic: string, title: string, description: string): Promise<string>;
  endLiveSession(topic: string, sessionId: string): Promise<void>;
  joinLiveSession(topic: string, sessionId: string): Promise<void>;
  leaveLiveSession(topic: string, sessionId: string): Promise<void>;
  listGameRooms(topic: string): Promise<GameRoomView[]>;
  createGameRoom(
    topic: string,
    title: string,
    description: string,
    participants: string[]
  ): Promise<string>;
  updateGameRoom(
    topic: string,
    roomId: string,
    status: GameRoomStatus,
    phaseLabel: string | null,
    scores: GameScoreView[]
  ): Promise<void>;
  getSyncStatus(): Promise<SyncStatus>;
  getDiscoveryConfig(): Promise<DiscoveryConfig>;
  getCommunityNodeConfig(): Promise<CommunityNodeConfig>;
  getCommunityNodeStatuses(): Promise<CommunityNodeNodeStatus[]>;
  setCommunityNodeConfig(baseUrls: string[]): Promise<CommunityNodeConfig>;
  clearCommunityNodeConfig(): Promise<void>;
  authenticateCommunityNode(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  clearCommunityNodeToken(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  getCommunityNodeConsentStatus(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  acceptCommunityNodeConsents(
    baseUrl: string,
    policySlugs: string[]
  ): Promise<CommunityNodeNodeStatus>;
  refreshCommunityNodeMetadata(baseUrl: string): Promise<CommunityNodeNodeStatus>;
  importPeerTicket(ticket: string): Promise<void>;
  setDiscoverySeeds(seedEntries: string[]): Promise<DiscoveryConfig>;
  unsubscribeTopic(topic: string): Promise<void>;
  getLocalPeerTicket(): Promise<string | null>;
  getBlobMediaPayload(hash: string, mime: string): Promise<BlobMediaPayload | null>;
  getBlobPreviewUrl(hash: string, mime: string): Promise<string | null>;
}

declare global {
  interface Window {
    __KUKURI_DESKTOP__?: DesktopApi;
  }
}

const BACKEND_UNAVAILABLE_MESSAGE = 'Desktop backend is not attached.';

function normalizeInvokeError(error: unknown): Error {
  const normalized =
    error instanceof Error
      ? error
      : typeof error === 'string'
        ? new Error(error)
        : typeof error === 'object' &&
            error !== null &&
            'message' in error &&
            typeof error.message === 'string'
          ? new Error(error.message)
          : new Error(BACKEND_UNAVAILABLE_MESSAGE);
  const message = normalized.message.toLowerCase();
  if (
    message.includes('__tauri') ||
    message.includes('__tauri_ipc__') ||
    (message.includes('ipc') && message.includes('not available')) ||
    (message.includes('invoke') && message.includes('undefined'))
  ) {
    return new Error(BACKEND_UNAVAILABLE_MESSAGE);
  }
  return normalized;
}

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw normalizeInvokeError(error);
  }
}

export const runtimeApi: DesktopApi = {
  createPost: async (topic, content, replyTo, attachments = []) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createPost(topic, content, replyTo, attachments);
    }
    return invokeDesktop<string>('create_post', {
      request: {
        topic,
        content,
        reply_to: replyTo,
        attachments,
      },
    });
  },
  listTimeline: async (topic, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listTimeline(topic, cursor, limit);
    }
    return invokeDesktop<TimelineView>('list_timeline', {
      request: {
        topic,
        cursor,
        limit,
      },
    });
  },
  listThread: async (topic, threadId, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listThread(topic, threadId, cursor, limit);
    }
    return invokeDesktop<TimelineView>('list_thread', {
      request: {
        topic,
        thread_id: threadId,
        cursor,
        limit,
      },
    });
  },
  listLiveSessions: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listLiveSessions(topic);
    }
    return invokeDesktop<LiveSessionView[]>('list_live_sessions', {
      request: {
        topic,
      },
    });
  },
  createLiveSession: async (topic, title, description) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createLiveSession(topic, title, description);
    }
    return invokeDesktop<string>('create_live_session', {
      request: {
        topic,
        title,
        description,
      },
    });
  },
  endLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.endLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('end_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  joinLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.joinLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('join_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  leaveLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.leaveLiveSession(topic, sessionId);
    }
    return invokeDesktop<void>('leave_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    });
  },
  listGameRooms: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listGameRooms(topic);
    }
    return invokeDesktop<GameRoomView[]>('list_game_rooms', {
      request: {
        topic,
      },
    });
  },
  createGameRoom: async (topic, title, description, participants) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createGameRoom(topic, title, description, participants);
    }
    return invokeDesktop<string>('create_game_room', {
      request: {
        topic,
        title,
        description,
        participants,
      },
    });
  },
  updateGameRoom: async (topic, roomId, status, phaseLabel, scores) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.updateGameRoom(topic, roomId, status, phaseLabel, scores);
    }
    return invokeDesktop<void>('update_game_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        phase_label: phaseLabel,
        scores,
      },
    });
  },
  getSyncStatus: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getSyncStatus();
    }
    return invokeDesktop<SyncStatus>('get_sync_status');
  },
  getDiscoveryConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getDiscoveryConfig();
    }
    return invokeDesktop<DiscoveryConfig>('get_discovery_config');
  },
  getCommunityNodeConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeConfig();
    }
    return invokeDesktop<CommunityNodeConfig>('get_community_node_config');
  },
  getCommunityNodeStatuses: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeStatuses();
    }
    return invokeDesktop<CommunityNodeNodeStatus[]>('get_community_node_statuses');
  },
  setCommunityNodeConfig: async (baseUrls) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setCommunityNodeConfig(baseUrls);
    }
    return invokeDesktop<CommunityNodeConfig>('set_community_node_config', {
      request: {
        base_urls: baseUrls,
      },
    });
  },
  clearCommunityNodeConfig: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.clearCommunityNodeConfig();
    }
    return invokeDesktop<void>('clear_community_node_config');
  },
  authenticateCommunityNode: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.authenticateCommunityNode(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('authenticate_community_node', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  clearCommunityNodeToken: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.clearCommunityNodeToken(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('clear_community_node_token', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  getCommunityNodeConsentStatus: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getCommunityNodeConsentStatus(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('get_community_node_consent_status', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  acceptCommunityNodeConsents: async (baseUrl, policySlugs) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.acceptCommunityNodeConsents(baseUrl, policySlugs);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('accept_community_node_consents', {
      request: {
        base_url: baseUrl,
        policy_slugs: policySlugs,
      },
    });
  },
  refreshCommunityNodeMetadata: async (baseUrl) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.refreshCommunityNodeMetadata(baseUrl);
    }
    return invokeDesktop<CommunityNodeNodeStatus>('refresh_community_node_metadata', {
      request: {
        base_url: baseUrl,
      },
    });
  },
  importPeerTicket: async (ticket) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importPeerTicket(ticket);
    }
    return invokeDesktop<void>('import_peer_ticket', {
      request: {
        ticket,
      },
    });
  },
  setDiscoverySeeds: async (seedEntries) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.setDiscoverySeeds(seedEntries);
    }
    return invokeDesktop<DiscoveryConfig>('set_discovery_seeds', {
      request: {
        seed_entries: seedEntries,
      },
    });
  },
  unsubscribeTopic: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.unsubscribeTopic(topic);
    }
    return invokeDesktop<void>('unsubscribe_topic', {
      request: {
        topic,
      },
    });
  },
  getLocalPeerTicket: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getLocalPeerTicket();
    }
    return invokeDesktop<string | null>('get_local_peer_ticket');
  },
  getBlobMediaPayload: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobMediaPayload(hash, mime);
    }
    return invokeDesktop<BlobMediaPayload | null>('get_blob_media_payload', {
      request: {
        hash,
        mime,
      },
    });
  },
  getBlobPreviewUrl: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobPreviewUrl(hash, mime);
    }
    return invokeDesktop<string | null>('get_blob_preview_url', {
      request: {
        hash,
        mime,
      },
    });
  },
};
