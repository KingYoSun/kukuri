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
