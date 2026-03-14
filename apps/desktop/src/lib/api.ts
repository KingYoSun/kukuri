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
  importPeerTicket(ticket: string): Promise<void>;
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

const unavailable = async (): Promise<never> => {
  throw new Error('Desktop backend is not attached.');
};

export const runtimeApi: DesktopApi = {
  createPost: async (topic, content, replyTo, attachments = []) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createPost(topic, content, replyTo, attachments);
    }
    return invoke<string>('create_post', {
      request: {
        topic,
        content,
        reply_to: replyTo,
        attachments,
      },
    }).catch(() => unavailable());
  },
  listTimeline: async (topic, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listTimeline(topic, cursor, limit);
    }
    return invoke<TimelineView>('list_timeline', {
      request: {
        topic,
        cursor,
        limit,
      },
    }).catch(() => unavailable());
  },
  listThread: async (topic, threadId, cursor, limit) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listThread(topic, threadId, cursor, limit);
    }
    return invoke<TimelineView>('list_thread', {
      request: {
        topic,
        thread_id: threadId,
        cursor,
        limit,
      },
    }).catch(() => unavailable());
  },
  listLiveSessions: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listLiveSessions(topic);
    }
    return invoke<LiveSessionView[]>('list_live_sessions', {
      request: {
        topic,
      },
    }).catch(() => unavailable());
  },
  createLiveSession: async (topic, title, description) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createLiveSession(topic, title, description);
    }
    return invoke<string>('create_live_session', {
      request: {
        topic,
        title,
        description,
      },
    }).catch(() => unavailable());
  },
  endLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.endLiveSession(topic, sessionId);
    }
    return invoke<void>('end_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    }).catch(() => unavailable());
  },
  joinLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.joinLiveSession(topic, sessionId);
    }
    return invoke<void>('join_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    }).catch(() => unavailable());
  },
  leaveLiveSession: async (topic, sessionId) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.leaveLiveSession(topic, sessionId);
    }
    return invoke<void>('leave_live_session', {
      request: {
        topic,
        session_id: sessionId,
      },
    }).catch(() => unavailable());
  },
  listGameRooms: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.listGameRooms(topic);
    }
    return invoke<GameRoomView[]>('list_game_rooms', {
      request: {
        topic,
      },
    }).catch(() => unavailable());
  },
  createGameRoom: async (topic, title, description, participants) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.createGameRoom(topic, title, description, participants);
    }
    return invoke<string>('create_game_room', {
      request: {
        topic,
        title,
        description,
        participants,
      },
    }).catch(() => unavailable());
  },
  updateGameRoom: async (topic, roomId, status, phaseLabel, scores) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.updateGameRoom(topic, roomId, status, phaseLabel, scores);
    }
    return invoke<void>('update_game_room', {
      request: {
        topic,
        room_id: roomId,
        status,
        phase_label: phaseLabel,
        scores,
      },
    }).catch(() => unavailable());
  },
  getSyncStatus: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getSyncStatus();
    }
    return invoke<SyncStatus>('get_sync_status').catch(() => unavailable());
  },
  importPeerTicket: async (ticket) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.importPeerTicket(ticket);
    }
    return invoke<void>('import_peer_ticket', {
      request: {
        ticket,
      },
    }).catch(() => unavailable());
  },
  unsubscribeTopic: async (topic) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.unsubscribeTopic(topic);
    }
    return invoke<void>('unsubscribe_topic', {
      request: {
        topic,
      },
    }).catch(() => unavailable());
  },
  getLocalPeerTicket: async () => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getLocalPeerTicket();
    }
    return invoke<string | null>('get_local_peer_ticket').catch(() => unavailable());
  },
  getBlobMediaPayload: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobMediaPayload(hash, mime);
    }
    return invoke<BlobMediaPayload | null>('get_blob_media_payload', {
      request: {
        hash,
        mime,
      },
    }).catch(() => unavailable());
  },
  getBlobPreviewUrl: async (hash, mime) => {
    if (window.__KUKURI_DESKTOP__) {
      return window.__KUKURI_DESKTOP__.getBlobPreviewUrl(hash, mime);
    }
    return invoke<string | null>('get_blob_preview_url', {
      request: {
        hash,
        mime,
      },
    }).catch(() => unavailable());
  },
};
