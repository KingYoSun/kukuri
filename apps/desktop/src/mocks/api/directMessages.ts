import {
  type AttachmentView,
  type DesktopApi,
  type DirectMessageMessageView,
  type DirectMessageTimelineView,
} from '@/lib/api';

import { type MockRuntime } from '../mockRuntime';

type DirectMessagesMock = Pick<
  DesktopApi,
  | 'openDirectMessage'
  | 'listDirectMessages'
  | 'listDirectMessageMessages'
  | 'sendDirectMessage'
  | 'deleteDirectMessageMessage'
  | 'clearDirectMessage'
  | 'getDirectMessageStatus'
>;

export function createDirectMessagesMock(runtime: MockRuntime): DirectMessagesMock {
  const {
    directMessageMessagesByPeer,
    openedDirectMessagePeers,
    syncStatus,
    directMessageStatusFor,
    directMessageConversationFor,
  } = runtime;

  return {
    async openDirectMessage(pubkey) {
      const status = directMessageStatusFor(pubkey);
      if (!status.send_enabled && !openedDirectMessagePeers.has(pubkey)) {
        throw new Error('direct message requires a mutual relationship');
      }
      openedDirectMessagePeers.add(pubkey);
      return directMessageConversationFor(pubkey);
    },
    async listDirectMessages() {
      return [...openedDirectMessagePeers]
        .map((pubkey) => directMessageConversationFor(pubkey))
        .sort(
          (left, right) =>
            (right.last_message_at ?? right.updated_at) - (left.last_message_at ?? left.updated_at) ||
            left.peer_pubkey.localeCompare(right.peer_pubkey)
        );
    },
    async listDirectMessageMessages(pubkey) {
      return {
        items: [...(directMessageMessagesByPeer[pubkey] ?? [])].sort(
          (left, right) => right.created_at - left.created_at || right.message_id.localeCompare(left.message_id)
        ),
        next_cursor: null,
      } satisfies DirectMessageTimelineView;
    },
    async sendDirectMessage(pubkey, text, attachments = [], replyToMessageId) {
      const status = directMessageStatusFor(pubkey);
      if (!status.send_enabled) {
        throw new Error('direct message requires a mutual relationship');
      }
      if (!text?.trim() && attachments.length === 0) {
        throw new Error('direct message requires text or attachment');
      }
      openedDirectMessagePeers.add(pubkey);
      runtime.sequence += 1;
      const messageId = `dm-${runtime.sequence}`;
      const messageAttachments: AttachmentView[] = attachments.map((attachment, index) => ({
        hash: `${messageId}-attachment-${index}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }));
      const nextMessage: DirectMessageMessageView = {
        dm_id: status.dm_id,
        message_id: messageId,
        sender_pubkey: syncStatus.local_author_pubkey,
        recipient_pubkey: pubkey,
        created_at: Date.now(),
        text: text?.trim() ?? '',
        reply_to_message_id: replyToMessageId ?? null,
        attachments: messageAttachments,
        outgoing: true,
        delivered: true,
      };
      directMessageMessagesByPeer[pubkey] = [
        nextMessage,
        ...(directMessageMessagesByPeer[pubkey] ?? []).filter(
          (message) => message.message_id !== nextMessage.message_id
        ),
      ];
      return messageId;
    },
    async deleteDirectMessageMessage(pubkey, messageId) {
      directMessageMessagesByPeer[pubkey] = (directMessageMessagesByPeer[pubkey] ?? []).filter(
        (message) => message.message_id !== messageId
      );
    },
    async clearDirectMessage(pubkey) {
      directMessageMessagesByPeer[pubkey] = [];
      openedDirectMessagePeers.add(pubkey);
    },
    async getDirectMessageStatus(pubkey) {
      return directMessageStatusFor(pubkey);
    },
  };
}
