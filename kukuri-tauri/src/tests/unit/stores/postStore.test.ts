import { describe, it, expect, beforeEach, vi } from 'vitest';

const { mockCreatePost, mockGetPosts } = vi.hoisted(() => ({
  mockCreatePost: vi.fn(),
  mockGetPosts: vi.fn(),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    createPost: mockCreatePost,
    getPosts: mockGetPosts,
    deletePost: vi.fn(),
    likePost: vi.fn(),
  },
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
    name: 'ユーザー1',
    displayName: 'ユーザー1',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockUser2 = {
    id: 'user2',
    pubkey: 'pubkey456',
    npub: 'npub456',
    name: 'ユーザー2',
    displayName: 'ユーザー2',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockUser3 = {
    id: 'user3',
    pubkey: 'pubkey789',
    npub: 'npub789',
    name: 'ユーザー3',
    displayName: 'ユーザー3',
    picture: '',
    about: '',
    nip05: '',
    avatar: null,
  };

  const mockPost1: Post = {
    id: 'post1',
    content: 'テスト投稿1',
    author: mockUser1,
    topicId: 'topic1',
    created_at: Date.now(),
    tags: [],
    likes: 0,
    replies: [],
  };

  const mockPost2: Post = {
    id: 'post2',
    content: 'テスト投稿2',
    author: mockUser2,
    topicId: 'topic1',
    created_at: Date.now() - 1000,
    tags: [],
    likes: 5,
    replies: [],
  };

  const mockPost3: Post = {
    id: 'post3',
    content: 'テスト投稿3',
    author: mockUser3,
    topicId: 'topic2',
    created_at: Date.now() - 2000,
    tags: [],
    likes: 10,
    replies: [],
  };

  beforeEach(async () => {
    mockCreatePost.mockReset();
    mockGetPosts.mockReset();
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

  it('初期状態が正しく設定されていること', () => {
    const state = usePostStore.getState();
    expect(state.posts.size).toBe(0);
    expect(state.postsByTopic.size).toBe(0);
  });

  it('setPostsメソッドが正しく動作すること', () => {
    usePostStore.getState().setPosts([mockPost1, mockPost2, mockPost3]);

    const state = usePostStore.getState();
    expect(state.posts.size).toBe(3);
    expect(state.postsByTopic.get('topic1')).toEqual(['post1', 'post2']);
    expect(state.postsByTopic.get('topic2')).toEqual(['post3']);
  });

  it('addPostメソッドが正しく動作すること', () => {
    usePostStore.getState().addPost(mockPost1);

    const state = usePostStore.getState();
    expect(state.posts.size).toBe(1);
    expect(state.posts.get('post1')).toEqual(mockPost1);
    expect(state.postsByTopic.get('topic1')).toEqual(['post1']);
  });

  it('updatePostメソッドが正しく動作すること', () => {
    usePostStore.setState({
      posts: new Map([['post1', mockPost1]]),
    });

    usePostStore.getState().updatePost('post1', { content: '更新された内容' });

    const state = usePostStore.getState();
    expect(state.posts.get('post1')?.content).toBe('更新された内容');
  });

  it('removePostメソッドが正しく動作すること', () => {
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

  it('addReplyメソッドが正しく動作すること', () => {
    const mockReplyUser = {
      id: 'user999',
      pubkey: 'pubkey999',
      npub: 'npub999',
      name: 'リプライユーザー',
      displayName: 'リプライユーザー',
      picture: '',
      about: '',
      nip05: '',
    };

    const reply: Post = {
      id: 'reply1',
      content: '返信テスト',
      author: mockReplyUser,
      topicId: 'topic1',
      created_at: Date.now(),
      tags: [],
      likes: 0,
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

  it('getPostsByTopicメソッドが正しく動作すること', () => {
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
    expect(topic1Posts[0].id).toBe('post1'); // 新しい順
    expect(topic1Posts[1].id).toBe('post2');

    const topic2Posts = usePostStore.getState().getPostsByTopic('topic2');
    expect(topic2Posts).toHaveLength(1);
    expect(topic2Posts[0].id).toBe('post3');

    const emptyPosts = usePostStore.getState().getPostsByTopic('nonexistent');
    expect(emptyPosts).toHaveLength(0);
  });

  it('オンライン時はcreatePostがTauri API経由で投稿すること', async () => {
    const apiResponse = {
      id: 'real-post',
      content: 'こんにちは',
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

    const result = await usePostStore.getState().createPost('こんにちは', 'topic1');

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenCalledWith({
      content: 'こんにちは',
      topic_id: 'topic1',
      reply_to: undefined,
      quoted_post: undefined,
    });

    const state = usePostStore.getState();
    expect(state.posts.get('real-post')?.content).toBe('こんにちは');
    expect(state.postsByTopic.get('topic1')).toEqual(['real-post']);
    expect(result.id).toBe('real-post');
  });

  it('replyToを指定するとreply_toパラメータを付与すること', async () => {
    const apiResponse = {
      id: 'reply-post',
      content: '返信本文',
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

    await usePostStore.getState().createPost('返信本文', 'topic1', { replyTo: 'event123' });

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenLastCalledWith({
      content: '返信本文',
      topic_id: 'topic1',
      reply_to: 'event123',
      quoted_post: undefined,
    });
  });

  it('quotedPostを指定するとquoted_postパラメータを付与すること', async () => {
    const apiResponse = {
      id: 'quote-post',
      content: '引用本文',
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

    await usePostStore.getState().createPost('引用本文', 'topic1', { quotedPost: 'note1' });

    expect(mockCreatePost).toHaveBeenCalledTimes(1);
    expect(mockCreatePost).toHaveBeenLastCalledWith({
      content: '引用本文',
      topic_id: 'topic1',
      reply_to: undefined,
      quoted_post: 'note1',
    });
  });

  it('fetchPostsがAPIレスポンスをストアに反映すること', async () => {
    const now = Math.floor(Date.now() / 1000);
    mockGetPosts.mockResolvedValueOnce([
      {
        id: 'api-post-1',
        content: 'P2Pからの投稿',
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
    expect(posts[0].content).toBe('P2Pからの投稿');
  });

  it('オンライン時に投稿を削除できること', async () => {
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

    await usePostStore.getState().deletePostRemote(mockPost1.id);

    expect(TauriApi.deletePost).toHaveBeenCalledWith(mockPost1.id);
    expect(usePostStore.getState().posts.has(mockPost1.id)).toBe(false);
    expect(usePostStore.getState().postsByTopic.get('topic1')).toEqual([]);
  });

  it('オフライン時は削除アクションとして保存されること', async () => {
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

    await usePostStore.getState().deletePostRemote(mockPost1.id);

    expect(saveOfflineActionMock).toHaveBeenCalledWith({
      userPubkey: mockUser1.pubkey,
      actionType: OfflineActionType.DELETE_POST,
      entityType: EntityType.POST,
      entityId: mockPost1.id,
      data: JSON.stringify({ postId: mockPost1.id }),
    });
    expect(TauriApi.deletePost).not.toHaveBeenCalled();
    expect(usePostStore.getState().posts.has(mockPost1.id)).toBe(false);

    getStateSpy.mockRestore();
    useOfflineStore.setState(originalOfflineState, true);
  });
});
