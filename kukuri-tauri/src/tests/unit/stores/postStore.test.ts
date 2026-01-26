import { describe, it, expect, beforeEach, vi } from 'vitest';

const { mockCreatePost, mockGetPosts, invalidatePostCachesMock } = vi.hoisted(() => ({
  mockCreatePost: vi.fn(),
  mockGetPosts: vi.fn(),
  invalidatePostCachesMock: vi.fn(),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    createPost: mockCreatePost,
    getPosts: mockGetPosts,
    deletePost: vi.fn(),
    likePost: vi.fn(),
  },
}));

vi.mock('@/lib/posts/cacheUtils', () => ({
  invalidatePostCaches: (...args: unknown[]) => invalidatePostCachesMock(...args),
}));

vi.mock('uuid', () => ({
  v4: () => 'temp-id',
}));

import { usePostStore } from '@/stores/postStore';
import { useOfflineStore } from '@/stores/offlineStore';
import { useAuthStore } from '@/stores/authStore';
import type { Post } from '@/stores/types';
import { OfflineActionType, EntityType } from '@/types/offline';

describe('postStore', () => {
  const mockUser1 = {
    id: 'user1',
    pubkey: 'pubkey123',
    npub: 'npub123',
    name: '繝ｦ繝ｼ繧ｶ繝ｼ1',
    displayName: '繝ｦ繝ｼ繧ｶ繝ｼ1',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockUser2 = {
    id: 'user2',
    pubkey: 'pubkey456',
    npub: 'npub456',
    name: '繝ｦ繝ｼ繧ｶ繝ｼ2',
    displayName: '繝ｦ繝ｼ繧ｶ繝ｼ2',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockUser3 = {
    id: 'user3',
    pubkey: 'pubkey789',
    npub: 'npub789',
    name: '繝ｦ繝ｼ繧ｶ繝ｼ3',
    displayName: '繝ｦ繝ｼ繧ｶ繝ｼ3',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockPost1: Post = {
    id: 'post1',
    content: '繝・せ繝域兜遞ｿ1',
    author: mockUser1,
    topicId: 'topic1',
    created_at: Date.now(),
    tags: [],
    likes: 0,
    boosts: 0,
    replies: [],
    replyCount: 0,
  };

  const mockPost2: Post = {
    id: 'post2',
    content: '繝・せ繝域兜遞ｿ2',
    author: mockUser2,
    topicId: 'topic1',
    created_at: Date.now() - 1000,
    tags: [],
    likes: 5,
    boosts: 0,
    replies: [],
    replyCount: 0,
  };

  const mockPost3: Post = {
    id: 'post3',
    content: '繝・せ繝域兜遞ｿ3',
    author: mockUser3,
    topicId: 'topic2',
    created_at: Date.now() - 2000,
    tags: [],
    likes: 10,
    boosts: 0,
    replies: [],
    replyCount: 0,
  };

  beforeEach(async () => {
    mockCreatePost.mockReset();
    mockGetPosts.mockReset();
    invalidatePostCachesMock.mockReset();
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.deletePost).mockReset();
    vi.mocked(TauriApi.likePost).mockReset();

    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    });

    useOfflineStore.setState({
      isOnline: true,
      pendingActions: [],
      optimisticUpdates: new Map(),
      syncErrors: new Map(),
      syncQueue: [],
      isSyncing: false,
    });

    useAuthStore.setState({
      isAuthenticated: true,
      currentUser: mockUser1,
      privateKey: 'test-private-key',
    });
  });

  it('蛻晄悄迥ｶ諷九′豁｣縺励￥險ｭ螳壹＆繧後※縺・ｋ縺薙→', () => {
    const state = usePostStore.getState();
    expect(state.posts.size).toBe(0);
    expect(state.postsByTopic.size).toBe(0);
  });

  it('setPosts繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    usePostStore.getState().setPosts([mockPost1, mockPost2, mockPost3]);

    const state = usePostStore.getState();
    expect(state.posts.size).toBe(3);
    expect(state.postsByTopic.get('topic1')).toEqual(['post1', 'post2']);
    expect(state.postsByTopic.get('topic2')).toEqual(['post3']);
  });

  it('addPost繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    usePostStore.getState().addPost(mockPost1);

    const state = usePostStore.getState();
    expect(state.posts.size).toBe(1);
    expect(state.posts.get('post1')).toEqual(mockPost1);
    expect(state.postsByTopic.get('topic1')).toEqual(['post1']);
  });

  it('updatePost繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    usePostStore.setState({
      posts: new Map([['post1', mockPost1]]),
    });

    usePostStore.getState().updatePost('post1', { content: '譖ｴ譁ｰ縺輔ｌ縺溷・螳ｹ' });

    const state = usePostStore.getState();
    expect(state.posts.get('post1')?.content).toBe('譖ｴ譁ｰ縺輔ｌ縺溷・螳ｹ');
  });

  it('removePost繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    usePostStore.setState({
      posts: new Map([
        ['post1', mockPost1],
        ['post2', mockPost2],
      ]),
      postsByTopic: new Map([['topic1', ['post1', 'post2']]]),
    });

    usePostStore.getState().removePost('post1');

    const state = usePostStore.getState();
    expect(state.posts.size).toBe(1);
    expect(state.posts.has('post1')).toBe(false);
    expect(state.postsByTopic.get('topic1')).toEqual(['post2']);
  });

  it('addReply繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    const mockReplyUser = {
      id: 'user999',
      pubkey: 'pubkey999',
      npub: 'npub999',
      name: '繝ｪ繝励Λ繧､繝ｦ繝ｼ繧ｶ繝ｼ',
      displayName: '繝ｪ繝励Λ繧､繝ｦ繝ｼ繧ｶ繝ｼ',
      picture: '',
      about: '',
      nip05: '',
    };

    const reply: Post = {
      id: 'reply1',
      content: 'reply content',
      author: mockReplyUser,
      topicId: 'topic1',
      created_at: Date.now(),
      tags: [],
      likes: 0,
      boosts: 0,
      replies: [],
    };

    usePostStore.setState({
      posts: new Map([['post1', mockPost1]]),
    });

    usePostStore.getState().addReply('post1', reply);

    const state = usePostStore.getState();
    const parentPost = state.posts.get('post1');
    expect(parentPost?.replies).toHaveLength(1);
    expect(parentPost?.replies?.[0]).toEqual(reply);
  });

  it('getPostsByTopic繝｡繧ｽ繝・ラ縺梧ｭ｣縺励￥蜍穂ｽ懊☆繧九％縺ｨ', () => {
    usePostStore.setState({
      posts: new Map([
        ['post1', mockPost1],
        ['post2', mockPost2],
        ['post3', mockPost3],
      ]),
      postsByTopic: new Map([
        ['topic1', ['post1', 'post2']],
        ['topic2', ['post3']],
      ]),
    });

    const topic1Posts = usePostStore.getState().getPostsByTopic('topic1');
    expect(topic1Posts).toHaveLength(2);
    expect(topic1Posts[0].id).toBe('post1'); // 譁ｰ縺励＞鬆・    expect(topic1Posts[1].id).toBe('post2');

    const topic2Posts = usePostStore.getState().getPostsByTopic('topic2');
    expect(topic2Posts).toHaveLength(1);
    expect(topic2Posts[0].id).toBe('post3');

    const emptyPosts = usePostStore.getState().getPostsByTopic('nonexistent');
    expect(emptyPosts).toHaveLength(0);
  });

  it('繧ｪ繝ｳ繝ｩ繧､繝ｳ譎ゅ・createPost縺卦auri API邨檎罰縺ｧ謚慕ｨｿ縺吶ｋ縺薙→', async () => {
    const apiResponse = {
      id: 'real-post',
      content: '縺薙ｓ縺ｫ縺｡縺ｯ',
      author_pubkey: 'pubkey123',
      author_npub: 'npub1pubkey123',
      topic_id: 'topic1',
      created_at: 1_725_000_000,
      likes: 0,
      boosts: 0,
      replies: 0,
      is_synced: true,
    };
    mockCreatePost.mockResolvedValueOnce(apiResponse);

    const result = await usePostStore.getState().createPost('縺薙ｓ縺ｫ縺｡縺ｯ', 'topic1');

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenCalledWith({
      content: '縺薙ｓ縺ｫ縺｡縺ｯ',
      topic_id: 'topic1',
      reply_to: undefined,
      quoted_post: undefined,
      scope: 'public',
    });

    const state = usePostStore.getState();
    expect(state.posts.get('real-post')?.content).toBe('縺薙ｓ縺ｫ縺｡縺ｯ');
    expect(state.postsByTopic.get('topic1')).toEqual(['real-post']);
    expect(result.id).toBe('real-post');
  });

  it('createPostがAPI失敗時にオフラインキューへ保存されること', async () => {
    mockCreatePost.mockRejectedValueOnce(new Error('network error'));
    const saveOfflineActionSpy = vi
      .spyOn(useOfflineStore.getState(), 'saveOfflineAction')
      .mockResolvedValue(undefined);

    const result = await usePostStore.getState().createPost('fallback body', 'topic1');
    const expectedTempId = 'temp-temp-id';

    expect(saveOfflineActionSpy).toHaveBeenCalledWith({
      userPubkey: mockUser1.pubkey,
      actionType: OfflineActionType.CREATE_POST,
      entityType: EntityType.POST,
      entityId: expectedTempId,
      data: JSON.stringify({
        content: 'fallback body',
        topicId: 'topic1',
        replyTo: undefined,
        quotedPost: undefined,
        scope: 'public',
      }),
    });
    const state = usePostStore.getState();
    expect(state.posts.has(expectedTempId)).toBe(true);
    expect(state.postsByTopic.get('topic1')).toEqual([expectedTempId]);
    expect(result.isSynced).toBe(false);

    saveOfflineActionSpy.mockRestore();
  });

  it('replyTo繧呈欠螳壹☆繧九→reply_to繝代Λ繝｡繝ｼ繧ｿ繧剃ｻ倅ｸ弱☆繧九％縺ｨ', async () => {
    const apiResponse = {
      id: 'reply-post',
      content: 'reply body',
      author_pubkey: 'pubkey123',
      author_npub: 'npub1pubkey123',
      topic_id: 'topic1',
      created_at: 1_725_000_100,
      likes: 0,
      boosts: 0,
      replies: 0,
      is_synced: true,
    };
    mockCreatePost.mockResolvedValueOnce(apiResponse);

    await usePostStore.getState().createPost('reply body', 'topic1', { replyTo: 'event123' });

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenLastCalledWith({
      content: 'reply body',
      topic_id: 'topic1',
      reply_to: 'event123',
      quoted_post: undefined,
      scope: 'public',
    });
  });

  it('quotedPost繧呈欠螳壹☆繧九→quoted_post繝代Λ繝｡繝ｼ繧ｿ繧剃ｻ倅ｸ弱☆繧九％縺ｨ', async () => {
    const apiResponse = {
      id: 'quote-post',
      content: 'quote body',
      author_pubkey: 'pubkey123',
      author_npub: 'npub1pubkey123',
      topic_id: 'topic1',
      created_at: 1_725_000_200,
      likes: 0,
      boosts: 0,
      replies: 0,
      is_synced: true,
    };
    mockCreatePost.mockResolvedValueOnce(apiResponse);

    await usePostStore.getState().createPost('quote body', 'topic1', { quotedPost: 'note1' });

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenLastCalledWith({
      content: 'quote body',
      topic_id: 'topic1',
      reply_to: undefined,
      quoted_post: 'note1',
      scope: 'public',
    });
  });

  it('fetchPosts縺窟PI繝ｬ繧ｹ繝昴Φ繧ｹ繧偵せ繝医い縺ｫ蜿肴丐縺吶ｋ縺薙→', async () => {
    const now = Math.floor(Date.now() / 1000);
    mockGetPosts.mockResolvedValueOnce([
      {
        id: 'api-post-1',
        content: 'P2P縺九ｉ縺ｮ謚慕ｨｿ',
        author_pubkey: 'pubkey999',
        author_npub: 'npub1pubkey999',
        topic_id: 'topic1',
        created_at: now,
        likes: 2,
        boosts: 0,
        replies: 0,
        is_synced: true,
      },
    ]);

    await usePostStore.getState().fetchPosts('topic1');

    expect(mockGetPosts).toHaveBeenCalledTimes(1);
    expect(mockGetPosts).toHaveBeenCalledWith({
      topic_id: 'topic1',
    });

    const posts = usePostStore.getState().getPostsByTopic('topic1');
    expect(posts).toHaveLength(1);
    expect(posts[0].id).toBe('api-post-1');
    expect(posts[0].content).toBe('P2P縺九ｉ縺ｮ謚慕ｨｿ');
  });

  it('繧ｪ繝ｳ繝ｩ繧､繝ｳ譎ゅ↓謚慕ｨｿ繧貞炎髯､縺ｧ縺阪ｋ縺薙→', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.deletePost).mockResolvedValue(undefined);

    usePostStore.setState({
      posts: new Map([
        [
          mockPost1.id,
          {
            ...mockPost1,
          },
        ],
      ]),
      postsByTopic: new Map([['topic1', [mockPost1.id]]]),
    });

    await usePostStore.getState().deletePostRemote({ id: mockPost1.id });

    expect(TauriApi.deletePost).toHaveBeenCalledWith(mockPost1.id);
    expect(usePostStore.getState().posts.has(mockPost1.id)).toBe(false);
    expect(usePostStore.getState().postsByTopic.get('topic1')).toEqual([]);
  });

  it('繧ｪ繝輔Λ繧､繝ｳ譎ゅ・蜑企勁繧｢繧ｯ繧ｷ繝ｧ繝ｳ縺ｨ縺励※菫晏ｭ倥＆繧後ｋ縺薙→', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.deletePost).mockResolvedValue(undefined);

    const originalOfflineState = useOfflineStore.getState();
    const saveOfflineActionMock = vi.fn().mockResolvedValue(undefined);
    const getStateSpy = vi.spyOn(useOfflineStore, 'getState').mockImplementation(() => ({
      ...originalOfflineState,
      isOnline: false,
      saveOfflineAction: saveOfflineActionMock,
    }));

    usePostStore.setState({
      posts: new Map([
        [
          mockPost1.id,
          {
            ...mockPost1,
          },
        ],
      ]),
      postsByTopic: new Map([['topic1', [mockPost1.id]]]),
    });

    await usePostStore.getState().deletePostRemote({ id: mockPost1.id });

    expect(saveOfflineActionMock).toHaveBeenCalledWith({
      userPubkey: mockUser1.pubkey,
      actionType: OfflineActionType.DELETE_POST,
      entityType: EntityType.POST,
      entityId: mockPost1.id,
      data: JSON.stringify({
        postId: mockPost1.id,
        topicId: mockPost1.topicId,
        authorPubkey: mockPost1.author.pubkey,
      }),
    });
    expect(TauriApi.deletePost).not.toHaveBeenCalled();
    expect(usePostStore.getState().posts.has(mockPost1.id)).toBe(false);

    getStateSpy.mockRestore();
    useOfflineStore.setState(originalOfflineState, true);
  });

  it('deletePostRemote 郢ｧ蜑・ｽｽ諛医・邵ｺ蜉ｱﾂｰ郢ｧ・ｭ郢晢ｽ｣郢昴・縺咏ｹ晢ｽ･郢ｧ蜑・ｽｺ閧ｲ・ｴ繝ｻ・邵ｺ・ｦ郢ｧ・ｭ郢晢ｽ｣郢昴・縺咏ｹ晢ｽ･郢ｧ螳夐亂陞滂ｽｮ邵ｺ謔滂ｿ｣陷峨・', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.deletePost).mockResolvedValue(undefined);

    usePostStore.setState({
      posts: new Map([[mockPost1.id, { ...mockPost1 }]]),
      postsByTopic: new Map([['topic1', [mockPost1.id]]]),
    });

    await usePostStore.getState().deletePostRemote({ id: mockPost1.id });

    expect(invalidatePostCachesMock).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        id: mockPost1.id,
        topicId: mockPost1.topicId,
        authorPubkey: mockPost1.author.pubkey,
      }),
    );
  });

  it('deletePostRemote 邵ｺ・ｯ郢ｧ・ｿ郢ｧ・､郢昴・繝ｻ郢ｧ・ｭ郢晢ｽ｣郢昴・縺咏ｹ晢ｽ･郢ｧ螳夲ｽｭ髢・ｮ螢ｹ・邵ｺ・ｾ邵ｺ・ｧ髯ｦ・ｨ驕会ｽｺ陷繝ｻ蟲ｩ陞滓じ竊鍋ｹｧ・ｭ郢晢ｽ｣郢昴・縺咏ｹ晢ｽ･郢ｧ螳夐亂陞滂ｽｮ邵ｺ謔滂ｿ｣陷峨・', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.deletePost).mockResolvedValue(undefined);

    usePostStore.setState({
      posts: new Map(),
      postsByTopic: new Map(),
    });

    await usePostStore.getState().deletePostRemote({
      id: 'missing-post',
      topicId: 'fallback-topic',
      authorPubkey: 'fallback-author',
    });

    expect(invalidatePostCachesMock).toHaveBeenCalledWith(
      expect.anything(),
      expect.objectContaining({
        id: 'missing-post',
        topicId: 'fallback-topic',
        authorPubkey: 'fallback-author',
      }),
    );
  });
});
