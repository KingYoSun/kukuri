import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { useTopicStore } from './topicStore';
import { p2pApi } from '@/lib/api/p2p';
import * as nostrApi from '@/lib/api/nostr';
import { errorHandler } from '@/lib/errorHandler';

// モック
vi.mock('@/lib/api/tauri');
vi.mock('@/lib/api/p2p');
vi.mock('@/lib/api/nostr');
vi.mock('@/lib/errorHandler');

// タイマーをモック
vi.useFakeTimers();

describe('topicStore', () => {
  beforeEach(() => {
    // ストアをリセット
    useTopicStore.setState({
      topics: new Map(),
      joinedTopics: [],
      currentTopic: null,
    });

    // モックをリセット
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  afterEach(() => {
    vi.clearAllMocks();
    vi.clearAllTimers();
  });

  describe('joinTopic', () => {
    it('トピックに参加し、P2P接続とNostrサブスクリプションを開始する', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      // P2P APIモック
      vi.mocked(p2pApi.joinTopic).mockResolvedValue(undefined);
      vi.mocked(nostrApi.subscribeToTopic).mockResolvedValue(undefined);

      // トピックに参加
      await joinTopic(topicId);

      // ストアが更新されることを確認
      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);

      // P2P接続が呼ばれることを確認
      expect(p2pApi.joinTopic).toHaveBeenCalledWith(topicId);

      // Nostrサブスクリプションが遅延して呼ばれることを確認
      expect(nostrApi.subscribeToTopic).not.toHaveBeenCalled();
      
      // 500ms待つ
      await vi.advanceTimersByTimeAsync(500);
      
      expect(nostrApi.subscribeToTopic).toHaveBeenCalledWith(topicId);
    });

    it('既に参加しているトピックには再参加しない', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      // 既に参加済みに設定
      useTopicStore.setState({
        joinedTopics: [topicId],
      });

      // トピックに参加を試みる
      await joinTopic(topicId);

      // APIが呼ばれないことを確認
      expect(p2pApi.joinTopic).not.toHaveBeenCalled();
      expect(nostrApi.subscribeToTopic).not.toHaveBeenCalled();
    });

    it('P2P接続に失敗した場合、ストアの状態を元に戻す', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      // P2P APIがエラーを返すようにモック
      vi.mocked(p2pApi.joinTopic).mockRejectedValue(new Error('P2P connection failed'));

      // トピックに参加を試みる
      await expect(joinTopic(topicId)).rejects.toThrow('P2P connection failed');

      // ストアが元に戻ることを確認
      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).not.toContain(topicId);

      // エラーハンドラーが呼ばれることを確認
      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to join topic',
        expect.any(Error),
        expect.objectContaining({
          context: 'TopicStore.joinTopic',
          showToast: true,
          toastTitle: 'トピックへの参加に失敗しました',
        })
      );
    });

    it('Nostrサブスクリプションが失敗してもP2P接続は維持される', async () => {
      const topicId = 'test-topic-1';
      const { joinTopic } = useTopicStore.getState();

      // P2P APIは成功、Nostr APIはエラーを返すようにモック
      vi.mocked(p2pApi.joinTopic).mockResolvedValue(undefined);
      vi.mocked(nostrApi.subscribeToTopic).mockRejectedValue(new Error('Nostr subscription failed'));

      // トピックに参加
      await joinTopic(topicId);

      // P2P接続は成功しているのでストアは更新される
      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);

      // タイマーを進める
      await vi.advanceTimersByTimeAsync(500);

      // エラーハンドラーが呼ばれることを確認（toastは表示しない）
      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to subscribe to Nostr topic',
        expect.any(Error),
        expect.objectContaining({
          context: 'TopicStore.joinTopic.nostrSubscribe',
          showToast: false,
        })
      );
    });
  });

  describe('leaveTopic', () => {
    it('トピックから離脱し、P2P接続を切断する', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      // 参加済みに設定
      useTopicStore.setState({
        joinedTopics: [topicId],
      });

      // P2P APIモック
      vi.mocked(p2pApi.leaveTopic).mockResolvedValue(undefined);

      // トピックから離脱
      await leaveTopic(topicId);

      // ストアが更新されることを確認
      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).not.toContain(topicId);

      // P2P切断が呼ばれることを確認
      expect(p2pApi.leaveTopic).toHaveBeenCalledWith(topicId);
    });

    it('参加していないトピックからは離脱しない', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      // トピックから離脱を試みる
      await leaveTopic(topicId);

      // APIが呼ばれないことを確認
      expect(p2pApi.leaveTopic).not.toHaveBeenCalled();
    });

    it('P2P切断に失敗した場合、ストアの状態を元に戻す', async () => {
      const topicId = 'test-topic-1';
      const { leaveTopic } = useTopicStore.getState();

      // 参加済みに設定
      useTopicStore.setState({
        joinedTopics: [topicId],
      });

      // P2P APIがエラーを返すようにモック
      vi.mocked(p2pApi.leaveTopic).mockRejectedValue(new Error('P2P disconnect failed'));

      // トピックから離脱を試みる
      await expect(leaveTopic(topicId)).rejects.toThrow('P2P disconnect failed');

      // ストアが元に戻ることを確認
      const { joinedTopics } = useTopicStore.getState();
      expect(joinedTopics).toContain(topicId);

      // エラーハンドラーが呼ばれることを確認
      expect(errorHandler.log).toHaveBeenCalledWith(
        'Failed to leave topic',
        expect.any(Error),
        expect.objectContaining({
          context: 'TopicStore.leaveTopic',
          showToast: true,
          toastTitle: 'トピックからの離脱に失敗しました',
        })
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
      };
      const { leaveTopic } = useTopicStore.getState();

      // 参加済みかつ現在のトピックに設定
      useTopicStore.setState({
        joinedTopics: [topicId],
        currentTopic: topic,
      });

      // P2P APIモック
      vi.mocked(p2pApi.leaveTopic).mockResolvedValue(undefined);

      // トピックから離脱
      await leaveTopic(topicId);

      // currentTopicがnullになることを確認
      const { currentTopic } = useTopicStore.getState();
      expect(currentTopic).toBeNull();
    });
  });
});