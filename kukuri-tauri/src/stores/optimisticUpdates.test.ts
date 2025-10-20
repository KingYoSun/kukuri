import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { usePostStore } from './postStore';
import { useTopicStore } from './topicStore';
import { useOfflineStore } from './offlineStore';
import { setupPersistMock } from './utils/testHelpers';
import { TauriApi } from '@/lib/api/tauri';
import { p2pApi } from '@/lib/api/p2p';
import { OfflineActionType, EntityType } from '@/types/offline';

// モック設定
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    createPost: vi.fn(),
    likePost: vi.fn(),
  },
}));

vi.mock('@/lib/api/p2p', () => ({
  p2pApi: {
    joinTopic: vi.fn(),
    leaveTopic: vi.fn(),
  },
}));

vi.mock('@/lib/api/nostr', () => ({
  subscribeToTopic: vi.fn(),
}));

vi.mock('@/api/offline', () => ({
  offlineApi: {
    saveOfflineAction: vi.fn().mockResolvedValue({
      localId: 'local-id-123',
      action: {
        id: 1,
        userPubkey: 'test-user-pubkey',
        actionType: 'test-action',
        targetId: 'test-target',
        actionData: '{}',
        localId: 'local-id-123',
        remoteId: undefined,
        isSynced: false,
        createdAt: Date.now(),
        syncedAt: undefined,
      },
    }),
  },
}));

describe('楽観的UI更新', () => {
  let localStorageMock: ReturnType<typeof setupPersistMock>;

  beforeEach(() => {
    localStorageMock = setupPersistMock();
    vi.clearAllMocks();
    localStorageMock.getItem.mockReturnValue('test-user-pubkey');

    // ストアのリセット
    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    });

    useTopicStore.setState({
      topics: new Map(),
      currentTopic: null,
      joinedTopics: [],
    });

    useOfflineStore.setState({
      isOnline: true,
      lastSyncedAt: undefined,
      pendingActions: [],
      syncQueue: [],
      optimisticUpdates: new Map(),
      isSyncing: false,
      syncErrors: new Map(),
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('投稿作成の楽観的更新', () => {
    it('オンライン時: 即座にUIに反映され、サーバー送信後に実際のIDに置き換えられる', async () => {
      const mockApiResponse = {
        id: 'real-post-id',
        content: 'テスト投稿',
        author_pubkey: 'test-user',
        topic_id: 'topic1',
        created_at: Date.now(),
        likes: 0,
        boosts: 0,
      };

      vi.mocked(TauriApi.createPost).mockResolvedValue(mockApiResponse);

      const createPost = usePostStore.getState().createPost;

      // 投稿を作成
      const createPromise = createPost('テスト投稿', 'topic1');

      // 即座にUIに反映されているか確認（楽観的更新）
      const storeState = usePostStore.getState();
      const tempPost = Array.from(storeState.posts.values()).find(
        (p) => p.content === 'テスト投稿',
      );

      expect(tempPost).toBeDefined();
      expect(tempPost?.id).toContain('temp-');
      expect(tempPost?.isSynced).toBe(false);

      // サーバー応答を待つ
      await createPromise;

      // 実際のIDに置き換えられているか確認
      const finalState = usePostStore.getState();
      const realPost = finalState.posts.get('real-post-id');

      expect(realPost).toBeDefined();
      expect(realPost?.id).toBe('real-post-id');
      expect(realPost?.isSynced).toBe(true);

      // 一時IDが削除されているか確認
      const tempPostAfter = Array.from(finalState.posts.values()).find((p) =>
        p.id.startsWith('temp-'),
      );
      expect(tempPostAfter).toBeUndefined();
    });

    it('オフライン時: 即座にUIに反映され、オフラインアクションとして保存される', async () => {
      useOfflineStore.setState({ isOnline: false });

      const offlineStore = useOfflineStore.getState();
      const saveOfflineActionSpy = vi.spyOn(offlineStore, 'saveOfflineAction');

      const createPost = usePostStore.getState().createPost;

      // 投稿を作成
      const result = await createPost('オフライン投稿', 'topic1');

      // 即座にUIに反映されているか確認
      expect(result.content).toBe('オフライン投稿');
      expect(result.id).toContain('temp-');
      expect(result.isSynced).toBe(false);

      // オフラインアクションとして保存されているか確認
      expect(saveOfflineActionSpy).toHaveBeenCalledWith({
        userPubkey: 'test-user-pubkey',
        actionType: OfflineActionType.CREATE_POST,
        entityType: EntityType.POST,
        entityId: expect.stringContaining('temp-'),
        data: expect.stringContaining('オフライン投稿'),
      });
    });

    it('サーバーエラー時: ロールバックされる', async () => {
      vi.mocked(TauriApi.createPost).mockRejectedValue(new Error('サーバーエラー'));

      const createPost = usePostStore.getState().createPost;

      // 投稿を作成（エラーになる）
      await expect(createPost('エラー投稿', 'topic1')).rejects.toThrow('サーバーエラー');

      // UIから削除されているか確認
      const storeState = usePostStore.getState();
      const errorPost = Array.from(storeState.posts.values()).find(
        (p) => p.content === 'エラー投稿',
      );

      expect(errorPost).toBeUndefined();
    });
  });

  describe('いいねの楽観的更新', () => {
    beforeEach(() => {
      // テスト用の投稿を追加
      usePostStore.setState({
        posts: new Map([
          [
            'post1',
            {
              id: 'post1',
              content: 'テスト投稿',
              author: {
                id: 'author1',
                pubkey: 'author1',
                npub: 'author1',
                name: 'テストユーザー',
                displayName: 'テストユーザー',
                about: '',
                picture: '',
                nip05: '',
              },
              topicId: 'topic1',
              created_at: Date.now(),
              tags: [],
              likes: 5,
              boosts: 0,
              replies: [],
              isSynced: true,
            },
          ],
        ]),
      });
    });

    it('オンライン時: 即座にいいね数が増加し、エラー時はロールバックされる', async () => {
      vi.mocked(TauriApi.likePost).mockResolvedValue(undefined);

      const likePost = usePostStore.getState().likePost;

      // いいねを実行
      await likePost('post1');

      // いいね数が増加しているか確認
      const post = usePostStore.getState().posts.get('post1');
      expect(post?.likes).toBe(6);
    });

    it('オフライン時: 即座にいいね数が増加し、オフラインアクションとして保存される', async () => {
      useOfflineStore.setState({ isOnline: false });

      const offlineStore = useOfflineStore.getState();
      const saveOfflineActionSpy = vi.spyOn(offlineStore, 'saveOfflineAction');

      const likePost = usePostStore.getState().likePost;

      // いいねを実行
      await likePost('post1');

      // いいね数が増加しているか確認
      const post = usePostStore.getState().posts.get('post1');
      expect(post?.likes).toBe(6);

      // オフラインアクションとして保存されているか確認
      expect(saveOfflineActionSpy).toHaveBeenCalledWith({
        userPubkey: 'test-user-pubkey',
        actionType: OfflineActionType.LIKE_POST,
        entityType: EntityType.POST,
        entityId: 'post1',
        data: JSON.stringify({ postId: 'post1' }),
      });
    });

    it('サーバーエラー時: いいね数がロールバックされる', async () => {
      vi.mocked(TauriApi.likePost).mockRejectedValue(new Error('サーバーエラー'));

      const likePost = usePostStore.getState().likePost;

      // いいねを実行（エラーになる）
      await expect(likePost('post1')).rejects.toThrow('サーバーエラー');

      // いいね数が元に戻っているか確認
      const post = usePostStore.getState().posts.get('post1');
      expect(post?.likes).toBe(5);
    });
  });

  describe('トピック参加の楽観的更新', () => {
    it('オンライン時: 即座に参加状態になり、P2P接続が実行される', async () => {
      vi.mocked(p2pApi.joinTopic).mockResolvedValue(undefined);

      const joinTopic = useTopicStore.getState().joinTopic;

      // トピックに参加
      const joinPromise = joinTopic('topic1');

      // 即座に参加状態になっているか確認
      expect(useTopicStore.getState().joinedTopics).toContain('topic1');

      await joinPromise;

      // P2P接続が実行されたか確認
      expect(p2pApi.joinTopic).toHaveBeenCalledWith('topic1');
    });

    it('オフライン時: 即座に参加状態になり、オフラインアクションとして保存される', async () => {
      useOfflineStore.setState({ isOnline: false });

      const offlineStore = useOfflineStore.getState();
      const saveOfflineActionSpy = vi.spyOn(offlineStore, 'saveOfflineAction');

      const joinTopic = useTopicStore.getState().joinTopic;

      // トピックに参加
      await joinTopic('topic1');

      // 即座に参加状態になっているか確認
      expect(useTopicStore.getState().joinedTopics).toContain('topic1');

      // オフラインアクションとして保存されているか確認
      expect(saveOfflineActionSpy).toHaveBeenCalledWith({
        userPubkey: 'test-user-pubkey',
        actionType: OfflineActionType.JOIN_TOPIC,
        entityType: EntityType.TOPIC,
        entityId: 'topic1',
        data: JSON.stringify({ topicId: 'topic1' }),
      });

      // P2P接続が実行されていないことを確認
      expect(p2pApi.joinTopic).not.toHaveBeenCalled();
    });

    it('エラー時: 参加状態がロールバックされる', async () => {
      vi.mocked(p2pApi.joinTopic).mockRejectedValue(new Error('接続エラー'));

      const joinTopic = useTopicStore.getState().joinTopic;

      // トピックに参加（エラーになる）
      await expect(joinTopic('topic1')).rejects.toThrow('接続エラー');

      // 参加状態がロールバックされているか確認
      expect(useTopicStore.getState().joinedTopics).not.toContain('topic1');
    });
  });

  describe('トピック離脱の楽観的更新', () => {
    beforeEach(() => {
      // 既に参加しているトピックを設定
      useTopicStore.setState({
        joinedTopics: ['topic1'],
      });
    });

    it('オンライン時: 即座に離脱状態になり、P2P切断が実行される', async () => {
      vi.mocked(p2pApi.leaveTopic).mockResolvedValue(undefined);

      const leaveTopic = useTopicStore.getState().leaveTopic;

      // トピックから離脱
      const leavePromise = leaveTopic('topic1');

      // 即座に離脱状態になっているか確認
      expect(useTopicStore.getState().joinedTopics).not.toContain('topic1');

      await leavePromise;

      // P2P切断が実行されたか確認
      expect(p2pApi.leaveTopic).toHaveBeenCalledWith('topic1');
    });

    it('オフライン時: 即座に離脱状態になり、オフラインアクションとして保存される', async () => {
      useOfflineStore.setState({ isOnline: false });

      const offlineStore = useOfflineStore.getState();
      const saveOfflineActionSpy = vi.spyOn(offlineStore, 'saveOfflineAction');

      const leaveTopic = useTopicStore.getState().leaveTopic;

      // トピックから離脱
      await leaveTopic('topic1');

      // 即座に離脱状態になっているか確認
      expect(useTopicStore.getState().joinedTopics).not.toContain('topic1');

      // オフラインアクションとして保存されているか確認
      expect(saveOfflineActionSpy).toHaveBeenCalledWith({
        userPubkey: 'test-user-pubkey',
        actionType: OfflineActionType.LEAVE_TOPIC,
        entityType: EntityType.TOPIC,
        entityId: 'topic1',
        data: JSON.stringify({ topicId: 'topic1' }),
      });

      // P2P切断が実行されていないことを確認
      expect(p2pApi.leaveTopic).not.toHaveBeenCalled();
    });

    it('エラー時: 離脱状態がロールバックされる', async () => {
      vi.mocked(p2pApi.leaveTopic).mockRejectedValue(new Error('切断エラー'));

      const leaveTopic = useTopicStore.getState().leaveTopic;

      // トピックから離脱（エラーになる）
      await expect(leaveTopic('topic1')).rejects.toThrow('切断エラー');

      // 離脱状態がロールバックされているか確認
      expect(useTopicStore.getState().joinedTopics).toContain('topic1');
    });
  });
});
