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
  created_at: number;
  reply_to?: string | null;
  root_id?: string | null;
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
  subscribed_topics: string[];
};

export interface DesktopApi {
  createPost(topic: string, content: string, replyTo?: string | null): Promise<string>;
  listTimeline(topic: string, cursor?: TimelineCursor | null, limit?: number): Promise<TimelineView>;
  listThread(
    topic: string,
    threadId: string,
    cursor?: TimelineCursor | null,
    limit?: number
  ): Promise<TimelineView>;
  getSyncStatus(): Promise<SyncStatus>;
  importPeerTicket(ticket: string): Promise<void>;
}

declare global {
  interface Window {
    __KUKURI_NEXT_DESKTOP__?: DesktopApi;
  }
}

const unavailable = async (): Promise<never> => {
  throw new Error('Desktop backend is not attached.');
};

export const runtimeApi: DesktopApi = {
  createPost: async (topic, content, replyTo) =>
    (window.__KUKURI_NEXT_DESKTOP__?.createPost(topic, content, replyTo) ?? unavailable()),
  listTimeline: async (topic, cursor, limit) =>
    (window.__KUKURI_NEXT_DESKTOP__?.listTimeline(topic, cursor, limit) ?? unavailable()),
  listThread: async (topic, threadId, cursor, limit) =>
    (window.__KUKURI_NEXT_DESKTOP__?.listThread(topic, threadId, cursor, limit) ?? unavailable()),
  getSyncStatus: async () => window.__KUKURI_NEXT_DESKTOP__?.getSyncStatus() ?? unavailable(),
  importPeerTicket: async (ticket) =>
    (window.__KUKURI_NEXT_DESKTOP__?.importPeerTicket(ticket) ?? unavailable()),
};
