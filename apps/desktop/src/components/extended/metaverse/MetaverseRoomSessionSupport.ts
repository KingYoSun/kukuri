import type { SyncStatus } from '@/lib/api';
import { METAVERSE_CHAT_BUBBLE_TTL_MS } from '../MetaverseSceneModel';
import type { LatestChatBubble, RoomChatMessage } from '../MetaverseSceneModel';

export function chatMessageFromApi(message: {
  room_id: string;
  message_id: string;
  author_peer_id: string;
  display_name?: string | null;
  body: string;
  created_at: number;
}): RoomChatMessage {
  return {
    roomId: message.room_id,
    messageId: message.message_id,
    authorPeerId: message.author_peer_id,
    displayName: message.display_name ?? null,
    body: message.body,
    createdAt: message.created_at,
  };
}

export function topicDiagnosticFor(syncStatus: SyncStatus, topic: string) {
  return syncStatus.topic_diagnostics.find(
    (diagnostic) => diagnostic.topic === topic || diagnostic.topic === `hint/${topic}`
  );
}

export function latestChatBubbleFromMessage(
  message: RoomChatMessage,
  now = Date.now()
): LatestChatBubble {
  return {
    peerId: message.authorPeerId,
    displayName: message.displayName ?? null,
    body: message.body,
    createdAt: message.createdAt,
    expiresAt: now + METAVERSE_CHAT_BUBBLE_TTL_MS,
  };
}
