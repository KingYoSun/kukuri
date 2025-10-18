import { invoke } from '@tauri-apps/api/core';

// Types
export interface NostrMetadata {
  name?: string;
  display_name?: string;
  about?: string;
  picture?: string;
  banner?: string;
  nip05?: string;
  lud16?: string;
  website?: string;
}

export interface RelayInfo {
  url: string;
  status: string;
}

export interface NostrEvent {
  id: string;
  author: string;
  content: string;
  created_at: number;
  kind: number;
  tags: string[][];
}

export interface NostrSubscriptionState {
  target: string;
  targetType: 'topic' | 'user';
  status: string;
  lastSyncedAt: number | null;
  lastAttemptAt: number | null;
  failureCount: number;
  errorMessage: string | null;
}

// Nostr Commands
export async function initializeNostr(): Promise<void> {
  return invoke('initialize_nostr');
}

export async function addRelay(url: string): Promise<void> {
  return invoke('add_relay', { url });
}

export async function publishTextNote(content: string): Promise<string> {
  return invoke('publish_text_note', { content });
}

export async function publishTopicPost(
  topicId: string,
  content: string,
  replyTo?: string,
): Promise<string> {
  return invoke('publish_topic_post', {
    topicId,
    content,
    replyTo: replyTo || null,
  });
}

export async function sendReaction(eventId: string, reaction: string): Promise<string> {
  return invoke('send_reaction', { eventId, reaction });
}

export async function updateNostrMetadata(metadata: NostrMetadata): Promise<string> {
  return invoke('update_nostr_metadata', { metadata });
}

export async function subscribeToTopic(topicId: string): Promise<void> {
  return invoke('subscribe_to_topic', { topicId });
}

export async function subscribeToUser(pubkey: string): Promise<void> {
  return invoke('subscribe_to_user', { pubkey });
}

export async function listNostrSubscriptions(): Promise<NostrSubscriptionState[]> {
  const response = await invoke<{
    subscriptions: {
      target: string;
      target_type: 'topic' | 'user';
      status: string;
      last_synced_at: number | null;
      last_attempt_at: number | null;
      failure_count: number;
      error_message: string | null;
    }[];
  }>('list_nostr_subscriptions');
  const raw = response.subscriptions ?? [];
  return raw.map((item) => ({
    target: item.target,
    targetType: item.target_type,
    status: item.status,
    lastSyncedAt: item.last_synced_at,
    lastAttemptAt: item.last_attempt_at,
    failureCount: item.failure_count,
    errorMessage: item.error_message,
  }));
}

export async function getNostrPubkey(): Promise<string | null> {
  return invoke('get_nostr_pubkey');
}

export async function deleteEvents(eventIds: string[], reason?: string): Promise<string> {
  return invoke('delete_events', { eventIds, reason });
}

export async function disconnectNostr(): Promise<void> {
  return invoke('disconnect_nostr');
}

export async function getRelayStatus(): Promise<RelayInfo[]> {
  return invoke('get_relay_status');
}
