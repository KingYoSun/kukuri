import { vi, describe, it, expect, beforeEach, MockedFunction } from 'vitest';
import * as nostrApi from '../nostr';

// Tauri APIをモック
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

const mockInvoke = invoke as MockedFunction<typeof invoke>;

describe('Nostr API', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('initializeNostr', () => {
    it('calls invoke with correct command', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await nostrApi.initializeNostr();

      expect(invoke).toHaveBeenCalledWith('initialize_nostr');
    });

    it('throws error when initialization fails', async () => {
      mockInvoke.mockRejectedValueOnce(new Error('Initialization failed'));

      await expect(nostrApi.initializeNostr()).rejects.toThrow('Initialization failed');
    });
  });

  describe('addRelay', () => {
    it('calls invoke with relay URL', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);
      const url = 'wss://relay.test';

      await nostrApi.addRelay(url);

      expect(invoke).toHaveBeenCalledWith('add_relay', { url });
    });
  });

  describe('publishTextNote', () => {
    it('returns event ID on success', async () => {
      const mockEventId = 'test-event-id-123';
      mockInvoke.mockResolvedValueOnce(mockEventId);

      const result = await nostrApi.publishTextNote('Test content');

      expect(invoke).toHaveBeenCalledWith('publish_text_note', {
        content: 'Test content',
      });
      expect(result).toBe(mockEventId);
    });
  });

  describe('publishTopicPost', () => {
    it('calls invoke with topic data without reply', async () => {
      const mockEventId = 'topic-event-id-456';
      mockInvoke.mockResolvedValueOnce(mockEventId);

      const result = await nostrApi.publishTopicPost('bitcoin', 'Bitcoin discussion');

      expect(invoke).toHaveBeenCalledWith('publish_topic_post', {
        topicId: 'bitcoin',
        content: 'Bitcoin discussion',
        replyTo: null,
      });
      expect(result).toBe(mockEventId);
    });

    it('calls invoke with topic data with reply', async () => {
      const mockEventId = 'topic-event-id-789';
      const replyToId = 'parent-event-id';
      mockInvoke.mockResolvedValueOnce(mockEventId);

      const result = await nostrApi.publishTopicPost('nostr', 'Reply content', replyToId);

      expect(invoke).toHaveBeenCalledWith('publish_topic_post', {
        topicId: 'nostr',
        content: 'Reply content',
        replyTo: replyToId,
      });
      expect(result).toBe(mockEventId);
    });
  });

  describe('sendReaction', () => {
    it('sends reaction to event', async () => {
      const mockReactionId = 'reaction-id-123';
      const targetEventId = 'target-event-id';
      mockInvoke.mockResolvedValueOnce(mockReactionId);

      const result = await nostrApi.sendReaction(targetEventId, '+');

      expect(invoke).toHaveBeenCalledWith('send_reaction', {
        eventId: targetEventId,
        reaction: '+',
      });
      expect(result).toBe(mockReactionId);
    });
  });

  describe('updateNostrMetadata', () => {
    it('updates user metadata', async () => {
      const mockEventId = 'metadata-event-id';
      const metadata: nostrApi.NostrMetadata = {
        name: 'Test User',
        about: 'Test about',
        picture: 'https://example.com/pic.jpg',
      };
      mockInvoke.mockResolvedValueOnce(mockEventId);

      const result = await nostrApi.updateNostrMetadata(metadata);

      expect(invoke).toHaveBeenCalledWith('update_nostr_metadata', {
        metadata,
      });
      expect(result).toBe(mockEventId);
    });
  });

  describe('subscribeToTopic', () => {
    it('subscribes to topic', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await nostrApi.subscribeToTopic('technology');

      expect(invoke).toHaveBeenCalledWith('subscribe_to_topic', {
        topicId: 'technology',
      });
    });
  });

  describe('subscribeToUser', () => {
    it('subscribes to user by public key', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);
      const pubkey = 'test-public-key-hex';

      await nostrApi.subscribeToUser(pubkey);

      expect(invoke).toHaveBeenCalledWith('subscribe_to_user', { pubkey });
    });
  });

  describe('getNostrPubkey', () => {
    it('returns public key when available', async () => {
      const mockPubkey = 'public-key-hex';
      mockInvoke.mockResolvedValueOnce(mockPubkey);

      const result = await nostrApi.getNostrPubkey();

      expect(invoke).toHaveBeenCalledWith('get_nostr_pubkey');
      expect(result).toBe(mockPubkey);
    });

    it('returns null when no public key', async () => {
      mockInvoke.mockResolvedValueOnce(null);

      const result = await nostrApi.getNostrPubkey();

      expect(result).toBeNull();
    });
  });

  describe('deleteEvents', () => {
    it('deletes events with reason', async () => {
      const mockDeletionId = 'deletion-event-id';
      const eventIds = ['event1', 'event2'];
      const reason = 'Spam';
      mockInvoke.mockResolvedValueOnce(mockDeletionId);

      const result = await nostrApi.deleteEvents(eventIds, reason);

      expect(invoke).toHaveBeenCalledWith('delete_events', {
        eventIds,
        reason,
      });
      expect(result).toBe(mockDeletionId);
    });

    it('deletes events without reason', async () => {
      const mockDeletionId = 'deletion-event-id';
      const eventIds = ['event1'];
      mockInvoke.mockResolvedValueOnce(mockDeletionId);

      const result = await nostrApi.deleteEvents(eventIds);

      expect(invoke).toHaveBeenCalledWith('delete_events', {
        eventIds,
        reason: undefined,
      });
      expect(result).toBe(mockDeletionId);
    });
  });

  describe('disconnectNostr', () => {
    it('disconnects from Nostr', async () => {
      mockInvoke.mockResolvedValueOnce(undefined);

      await nostrApi.disconnectNostr();

      expect(invoke).toHaveBeenCalledWith('disconnect_nostr');
    });
  });

  describe('getRelayStatus', () => {
    it('returns relay status information', async () => {
      const mockRelayStatus: nostrApi.RelayInfo[] = [
        { url: 'wss://relay1.test', status: 'connected' },
        { url: 'wss://relay2.test', status: 'disconnected' },
      ];
      mockInvoke.mockResolvedValueOnce(mockRelayStatus);

      const result = await nostrApi.getRelayStatus();

      expect(invoke).toHaveBeenCalledWith('get_relay_status');
      expect(result).toEqual(mockRelayStatus);
    });

    it('returns empty array when no relays', async () => {
      mockInvoke.mockResolvedValueOnce([]);

      const result = await nostrApi.getRelayStatus();

      expect(result).toEqual([]);
    });
  });
});
