import { invokeCommand, invokeCommandVoid } from '@/lib/api/tauriClient';

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
interface EventCommandResponse {
  event_id: string;
  success: boolean;
  message?: string | null;
}

export async function initializeNostr(): Promise<void> {
  await invokeCommandVoid('initialize_nostr');
}

export async function addRelay(url: string): Promise<void> {
  await invokeCommandVoid('add_relay', { url });
}

export async function publishTextNote(content: string): Promise<string> {
  const response = await invokeCommand<EventCommandResponse>('publish_text_note', { content });
  return response.event_id;
}

export async function publishTopicPost(
  topicId: string,
  content: string,
  replyTo?: string,
): Promise<string> {
  const response = await invokeCommand<EventCommandResponse>('publish_topic_post', {
    topicId,
    content,
    replyTo: replyTo ?? null,
  });
  return response.event_id;
}

export async function sendReaction(eventId: string, reaction: string): Promise<string> {
  const response = await invokeCommand<EventCommandResponse>('send_reaction', {
    eventId,
    reaction,
  });
  return response.event_id;
}

export async function updateNostrMetadata(metadata: NostrMetadata): Promise<string> {
  const response = await invokeCommand<EventCommandResponse>('update_nostr_metadata', {
    metadata,
  });
  return response.event_id;
}

export async function subscribeToTopic(topicId: string): Promise<void> {
  await invokeCommandVoid('subscribe_to_topic', { topicId });
}

export async function subscribeToUser(pubkey: string): Promise<void> {
  await invokeCommandVoid('subscribe_to_user', { pubkey });
}

export async function listNostrSubscriptions(): Promise<NostrSubscriptionState[]> {
  const response = await invokeCommand<{
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
  const response = await invokeCommand<{ pubkey: string | null }>('get_nostr_pubkey');
  return response.pubkey ?? null;
}

export async function deleteEvents(eventIds: string[], reason?: string): Promise<string> {
  const response = await invokeCommand<EventCommandResponse>('delete_events', {
    eventIds,
    reason,
  });
  return response.event_id;
}

export async function disconnectNostr(): Promise<void> {
  await invokeCommandVoid('disconnect_nostr');
}

export async function getRelayStatus(): Promise<RelayInfo[]> {
  return invokeCommand<RelayInfo[]>('get_relay_status');
}
