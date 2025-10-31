import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { useTopicStore } from '@/stores/topicStore';
import type { Topic } from '@/stores/types';
import { TauriApi } from '@/lib/api/tauri';
import * as nostrApi from '@/lib/api/nostr';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/lib/api/tauri');
vi.mock('@/lib/api/nostr');
vi.mock('@/lib/errorHandler');

vi.useFakeTimers();

const resetStore = () => {
  useTopicStore.setState({
    topics: new Map(),
    joinedTopics: [],
    currentTopic: null,
    topicUnreadCounts: new Map(),
    topicLastReadAt: new Map(),
  });
};

describe('useTopicStore', () => {
  const mockTopic1: Topic = {
    id: 'topic1',
    name: 'テストトピック1',
    description: '説明1',
    tags: ['tag1'],
    memberCount: 100,
    postCount: 50,
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };

  const mockTopic2: Topic = {
    id: 'topic2',
    name: 'テストトピック2',
    description: '説明2',
    tags: ['tag2'],
    memberCount: 200,
    postCount: 100,
    lastActive: Date.now(),
    isActive: true,
    createdAt: new Date(),
  };

  beforeEach(() => {
    resetStore();
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  describe('state management helpers', () => {
    it('初期状態が正しく設定されていること', () => {
      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(0);
      expect(state.currentTopic).toBeNull();
      expect(state.joinedTopics).toEqual([]);
      expect(state.topicUnreadCounts.size).toBe(0);
      expect(state.topicLastReadAt.size).toBe(0);
    });

    it('setTopicsメソッドが正しく動作すること', () => {
      useTopicStore.getState().setTopics([mockTopic1, mockTopic2]);

      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(2);
      expect(state.topics.get('topic1')).toEqual(mockTopic1);
      expect(state.topics.get('topic2')).toEqual(mockTopic2);
    });

    it('addTopicメソッドが正しく動作すること', () => {
      useTopicStore.getState().addTopic(mockTopic1);

      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(1);
      expect(state.topics.get('topic1')).toEqual(mockTopic1);
    });

    it('updateTopicメソッドが正しく動作すること', () => {
      useTopicStore.setState({
        topics: new Map([['topic1', mockTopic1]]),
      });

      useTopicStore.getState().updateTopic('topic1', { memberCount: 150 });

      const state = useTopicStore.getState();
      expect(state.topics.get('topic1')?.memberCount).toBe(150);
    });

    it('removeTopicメソッドが正しく動作すること', () => {
      useTopicStore.setState({
        topics: new Map([
          ['topic1', mockTopic1],
          ['topic2', mockTopic2],
        ]),
        currentTopic: mockTopic1,
        topicUnreadCounts: new Map([
          ['topic1', 2],
          ['topic2', 0],
        ]),
        topicLastReadAt: new Map([['topic1', 123]]),
      });

      useTopicStore.getState().removeTopic('topic1');

      const state = useTopicStore.getState();
      expect(state.topics.size).toBe(1);
      expect(state.topics.has('topic1')).toBe(false);
      expect(state.currentTopic).toBeNull();
      expect(state.topicUnreadCounts.has('topic1')).toBe(false);
      expect(state.topicLastReadAt.has('topic1')).toBe(false);
    });

    it('setCurrentTopicメソッドが正しく動作すること', () => {
      useTopicStore.getState().setCurrentTopic(mockTopic1);

      const state = useTopicStore.getState();
      expect(state.currentTopic).toEqual(mockTopic1);
      expect(state.topicUnreadCounts.get(mockTopic1.id)).toBe(0);
      expect(state.topicLastReadAt.has(mockTopic1.id)).toBe(true);
    });

    it('handleIncomingTopicMessageが未読件数を増加させること', () => {
      useTopicStore.setState({
        currentTopic: null,
        topicUnreadCounts: new Map([['topic1', 0]]),
      });

      useTopicStore.getState().handleIncomingTopicMessage('topic1', Date.now());

      expect(useTopicStore.getState().topicUnreadCounts.get('topic1')).toBe(1);
    });

    it('ハンドラが閲覧中のトピックでは未読件数をリセットすること', () => {
      useTopicStore.setState({
        currentTopic: mockTopic1,
        topicUnreadCounts: new Map([['topic1', 3]]),
      });

      useTopicStore.getState().handleIncomingTopicMessage('topic1', Date.now());

      expect(useTopicStore.getState().topicUnreadCounts.get('topic1')).toBe(0);
    });

    it('handleIncomingTopicMessageがミリ秒タイムスタンプを秒へ正規化すること', () => {
      const nowMs = Date.now();
      useTopicStore.setState({
        currentTopic: mockTopic1,
        topicUnreadCounts: new Map([['topic1', 1]]),
        topicLastReadAt: new Map([['topic1', 0]]),
      });

      useTopicStore.getState().handleIncomingTopicMessage('topic1', nowMs);

      const stored = useTopicStore.getState().topicLastReadAt.get('topic1');
      expect(stored).toBe(Math.floor(nowMs / 1000));
    });

    it('joinTopicメソッドが重複を許容せず未読カウントを初期化すること', async () => {
      vi.mocked(TauriApi.joinTopic).mockResolvedValue(undefined);

      const { joinTopic } = useTopicStore.getState();
      await joinTopic('topic1');
      await joinTopic('topic2');
      await joinTopic('topic1'); // 重複

      const state = useTopicStore.getState();
      expect(state.joinedTopics).toEqual(['topic1', 'topic2']);
      expect(state.topicUnreadCounts.get('topic1')).toBe(0);
      expect(state.topicUnreadCounts.get('topic2')).toBe(0);
    });

    it('leaveTopicメソッドが現在のトピックを解除すること', async () => {
      const { leaveTopic } = useTopicStore.getState();
      useTopicStore.setState({
        joinedTopics: ['topic1', 'topic2'],
        currentTopic: mockTopic1,
      });

      await leaveTopic('topic1');

      const state = useTopicStore.getState();
      expect(state.joinedTopics).toEqual(['topic2']);
      expect(state.currentTopic).toBeNull();
    });
  });

  describe('joinTopic', () => {
    it('トピックに参加し、P2P接続とNostrサブスクリプションを開始する', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      vi.mocked(TauriApi.joinTopic).mockResolvedValue(undefined);
      vi.mocked(nostrApi.subscribeToTopic).mockResolvedValue(undefined);

      await joinTopic(topicId);

      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);
      expect(TauriApi.joinTopic).toHaveBeenCalledWith(topicId);
      expect(nostrApi.subscribeToTopic).not.toHaveBeenCalled();

      await vi.advanceTimersByTimeAsync(500);

      expect(nostrApi.subscribeToTopic).toHaveBeenCalledWith(topicId);
    });

    it('既に参加しているトピックには再参加しない', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      useTopicStore.setState({
        joinedTopics: [topicId],
      });

      await joinTopic(topicId);

      expect(TauriApi.joinTopic).not.toHaveBeenCalled();
      expect(nostrApi.subscribeToTopic).not.toHaveBeenCalled();
    });

    it('P2P接続に失敗した場合、ストアの状態を元に戻す', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      const error = new Error('P2P connection failed');
      vi.mocked(TauriApi.joinTopic).mockRejectedValue(error);

      await expect(joinTopic(topicId)).rejects.toThrow('P2P connection failed');

      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).not.toContain(topicId);

      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to join topic',
        error,
        expect.objectContaining({
          context: 'TopicStore.joinTopic',
          showToast: true,
          toastTitle: 'トピックへの参加に失敗しました',
        }),
      );
    });

    it('Nostrサブスクリプションが失敗してもP2P接続は維持される', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      vi.mocked(TauriApi.joinTopic).mockResolvedValue(undefined);
      const nostrError = new Error('Nostr subscription failed');
      vi.mocked(nostrApi.subscribeToTopic).mockRejectedValue(nostrError);

      await joinTopic(topicId);

      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);

      await vi.advanceTimersByTimeAsync(500);

      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to subscribe to Nostr topic',
        nostrError,
        expect.objectContaining({
          context: 'TopicStore.joinTopic.nostrSubscribe',
          showToast: false,
        }),
      );
    });
  });

  describe('leaveTopic', () => {
    it('トピックから離脱し、P2P接続を切断する', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      useTopicStore.setState({
        joinedTopics: [topicId],
        topicUnreadCounts: new Map([[topicId, 3]]),
        topicLastReadAt: new Map([[topicId, 99]]),
      });

      vi.mocked(TauriApi.leaveTopic).mockResolvedValue(undefined);

      await leaveTopic(topicId);

      const { joinedTopics, topicUnreadCounts, topicLastReadAt } = useTopicStore.getState();
      expect(joinedTopics).not.toContain(topicId);
      expect(TauriApi.leaveTopic).toHaveBeenCalledWith(topicId);
      expect(topicUnreadCounts.has(topicId)).toBe(false);
      expect(topicLastReadAt.has(topicId)).toBe(false);
    });

    it('参加していないトピックからは離脱しない', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      await leaveTopic(topicId);

      expect(TauriApi.leaveTopic).not.toHaveBeenCalled();
    });

    it('P2P切断に失敗した場合、ストアの状態を元に戻す', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      useTopicStore.setState({
        joinedTopics: [topicId],
        topicUnreadCounts: new Map([[topicId, 4]]),
        topicLastReadAt: new Map([[topicId, 55]]),
      });

      const error = new Error('P2P disconnect failed');
      vi.mocked(TauriApi.leaveTopic).mockRejectedValue(error);

      await expect(leaveTopic(topicId)).rejects.toThrow('P2P disconnect failed');

      const { joinedTopics, topicUnreadCounts, topicLastReadAt } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);
      expect(topicUnreadCounts.get(topicId)).toBe(4);
      expect(topicLastReadAt.get(topicId)).toBe(55);

      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to leave topic',
        error,
        expect.objectContaining({
          context: 'TopicStore.leaveTopic',
          showToast: true,
          toastTitle: 'トピックからの離脱に失敗しました',
        }),
      );
    });

    it('現在のトピックから離脱した場合、currentTopicをnullにする', async () => {
      const topicId = 'test-topic-1';
      const topic = {
        id: topicId,
        name: 'Test Topic',
        description: 'Test description',
        createdAt: new Date(),
        memberCount: 0,
        postCount: 0,
        isActive: true,
        tags: [],
      } satisfies Topic;
      const { leaveTopic } = useTopicStore.getState();

      useTopicStore.setState({
        joinedTopics: [topicId],
        currentTopic: topic,
        topicUnreadCounts: new Map([[topicId, 1]]),
        topicLastReadAt: new Map([[topicId, 10]]),
      });

      vi.mocked(TauriApi.leaveTopic).mockResolvedValue(undefined);

      await leaveTopic(topicId);

      const { currentTopic, topicUnreadCounts, topicLastReadAt } = useTopicStore.getState();
      expect(currentTopic).toBeNull();
      expect(topicUnreadCounts.has(topicId)).toBe(false);
      expect(topicLastReadAt.has(topicId)).toBe(false);
    });
  });
});
