/**
 * WP-S5: useDesktopShellActions の characterization テスト(ハンドラ単位)。
 *
 * 後続 WP-H6(shell store のスライス化・selector 化・prop drilling 除去)の
 * 安全網として「現時点で観測した挙動」をそのまま固定する。
 * このテストが落ちた場合、疑うべきは加えた変更でありテストではない。
 *
 * 方針:
 * - 引数 21 個は全て vi.fn / mock api / stub で注入し、返り値のハンドラ単位で
 *   「操作 → api 呼び出し引数 + store 状態遷移」を固定する。
 * - translate は key をそのまま返す stub を注入し、composerError 等の期待値は
 *   訳キー文字列で assert する(locale リソースの変更で割れないようにする)。
 * - ハンドラの参照同一性・返り値 58 キーの全量比較・snapshot は書かない
 *   (H6 のリファクタで無意味に割れるため)。固定するのは api 呼び出し引数
 *   (toHaveBeenCalledWith)・store 状態遷移・キー/フィールド単位の値のみ。
 */
import { act, renderHook, waitFor } from '@testing-library/react';
import type { FormEvent } from 'react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type {
  BookmarkedPostView,
  DesktopApi,
  PostView,
  ReactionStateView,
  RecentReactionView,
} from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type { DesktopShellState, DraftMediaItem } from '@/shell/store';
import { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import {
  createShellHookHarness,
  resetWindowHash,
} from '@/shell/testSupport/renderShellHook';

const SELF_PUBKEY = 'f'.repeat(64);
const AUTHOR_A_PUBKEY = 'a'.repeat(64);

// key をそのまま返す stub。文言 assert を locale リソースから切り離す。
const stubTranslate = (key: string) => key;

function buildPost(overrides: Partial<PostView> = {}): PostView {
  return {
    object_id: 'post-1',
    envelope_id: 'envelope-post-1',
    author_pubkey: AUTHOR_A_PUBKEY,
    author_name: null,
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'hello world',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-1',
    channel_id: null,
    audience_label: 'Public',
    ...overrides,
  };
}

// api を deferred で凍結し、optimistic post の 'pending' 状態を決定的に観測する。
function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

// handlePublish は FormEvent を受けるため preventDefault 記録付きの stub を渡す。
function publishFormEvent() {
  const preventDefault = vi.fn();
  return {
    event: { preventDefault } as unknown as FormEvent<HTMLFormElement>,
    preventDefault,
  };
}

function directMessageFormEvent(value: string) {
  const preventDefault = vi.fn();
  return {
    event: {
      preventDefault,
      currentTarget: {
        querySelector: () => ({ value }),
      },
    } as unknown as FormEvent<HTMLFormElement>,
    preventDefault,
  };
}

type RenderActionsOptions = {
  /** mock api のうち差し替えたいメソッドだけを vi.fn で上書きする。 */
  api?: Partial<DesktopApi>;
  /** render 前の store プリセット(act 不要)。current を受けて patch を返す。 */
  preset?: (current: DesktopShellState) => Partial<DesktopShellState>;
};

// 21 引数を全て vi.fn / mock api / stub で配線する共通ハーネス。
function renderActionsHook(options: RenderActionsOptions = {}) {
  const harness = createShellHookHarness();
  if (options.preset) {
    harness.store.getState().patchState(options.preset(harness.store.getState()));
  }

  const api: DesktopApi = { ...createDesktopMockApi(), ...options.api };
  const loadTopics = vi.fn(async () => undefined);
  const refreshVisibleTimelineAfterPublish = vi.fn(async () => undefined);
  const syncRoute = vi.fn();
  const openDirectMessagePane = vi.fn(async () => undefined);
  const openAuthorDetail = vi.fn(async () => undefined);
  const openThread = vi.fn(async () => undefined);
  const setComposeDialogOpen = vi.fn();
  const setLiveCreateDialogOpen = vi.fn();
  const setGameCreateDialogOpen = vi.fn();
  const setProfileAvatarPreviewUrl = vi.fn();
  const setProfileAvatarInputKey = vi.fn();
  const releaseDraftPreview = vi.fn();
  const releaseAllDraftPreviews = vi.fn();
  const rememberDraftPreview = vi.fn();
  const releaseDirectMessageDraftPreview = vi.fn();
  const releaseAllDirectMessageDraftPreviews = vi.fn();
  const rememberDirectMessageDraftPreview = vi.fn();
  const buildImageDraftItem = vi.fn(
    async (file: File): Promise<DraftMediaItem> => ({
      id: `image-item-${file.name}`,
      source_name: file.name,
      preview_url: `blob:image-item-${file.name}`,
      attachments: [],
    })
  );
  const buildVideoDraftItem = vi.fn(
    async (file: File): Promise<DraftMediaItem> => ({
      id: `video-item-${file.name}`,
      source_name: file.name,
      preview_url: `blob:video-item-${file.name}`,
      attachments: [],
    })
  );

  const rendered = renderHook(
    () =>
      useDesktopShellActions({
        api,
        translate: stubTranslate,
        loadTopics,
        refreshVisibleTimelineAfterPublish,
        syncRoute,
        openDirectMessagePane,
        openAuthorDetail,
        openThread,
        setComposeDialogOpen,
        setLiveCreateDialogOpen,
        setGameCreateDialogOpen,
        setProfileAvatarPreviewUrl,
        setProfileAvatarInputKey,
        releaseDraftPreview,
        releaseAllDraftPreviews,
        rememberDraftPreview,
        releaseDirectMessageDraftPreview,
        releaseAllDirectMessageDraftPreviews,
        rememberDirectMessageDraftPreview,
        buildImageDraftItem,
        buildVideoDraftItem,
      }),
    { wrapper: harness.wrapper }
  );

  return {
    ...rendered,
    store: harness.store,
    api,
    mocks: {
      loadTopics,
      refreshVisibleTimelineAfterPublish,
      syncRoute,
      openDirectMessagePane,
      openAuthorDetail,
      openThread,
      setComposeDialogOpen,
      setLiveCreateDialogOpen,
      setGameCreateDialogOpen,
      setProfileAvatarPreviewUrl,
      setProfileAvatarInputKey,
      releaseDraftPreview,
      releaseAllDraftPreviews,
      rememberDraftPreview,
      releaseDirectMessageDraftPreview,
      releaseAllDirectMessageDraftPreviews,
      rememberDirectMessageDraftPreview,
      buildImageDraftItem,
      buildVideoDraftItem,
    },
  };
}

beforeEach(() => {
  resetWindowHash();
});

describe('useDesktopShellActions', () => {
  test('DM mutations use localized fallbacks for non-Error failures', async () => {
    const peerPubkey = 'd'.repeat(64);
    const sendDirectMessage = vi.fn().mockRejectedValue(null);
    const deleteDirectMessageMessage = vi.fn().mockRejectedValue(null);
    const clearDirectMessage = vi.fn().mockRejectedValue(null);
    const view = renderActionsHook({
      api: { sendDirectMessage, deleteDirectMessageMessage, clearDirectMessage },
      preset: (current) => ({
        selectedDirectMessagePeerPubkey: peerPubkey,
        directMessageComposer: 'hello',
        syncStatus: { ...current.syncStatus, local_author_pubkey: SELF_PUBKEY },
      }),
    });
    const { event } = directMessageFormEvent('hello');

    await act(async () => view.result.current.handleSendDirectMessage(event));
    expect(view.store.getState().directMessageError).toBe(
      'common:errors.failedToSendDirectMessage'
    );

    await act(async () =>
      view.result.current.handleDeleteDirectMessageMessage(peerPubkey, 'message-1')
    );
    expect(view.store.getState().directMessageError).toBe(
      'common:errors.failedToDeleteDirectMessage'
    );

    await act(async () => view.result.current.handleClearDirectMessage(peerPubkey));
    expect(view.store.getState().directMessageError).toBe(
      'common:errors.failedToClearDirectMessages'
    );
  });

  test('DM mutations preserve concrete Error messages', async () => {
    const peerPubkey = 'd'.repeat(64);
    const view = renderActionsHook({
      api: {
        sendDirectMessage: vi.fn().mockRejectedValue(new Error('send failed concretely')),
        deleteDirectMessageMessage: vi
          .fn()
          .mockRejectedValue(new Error('delete failed concretely')),
        clearDirectMessage: vi.fn().mockRejectedValue(new Error('clear failed concretely')),
      },
      preset: (current) => ({
        selectedDirectMessagePeerPubkey: peerPubkey,
        directMessageComposer: 'hello',
        syncStatus: { ...current.syncStatus, local_author_pubkey: SELF_PUBKEY },
      }),
    });
    const { event } = directMessageFormEvent('hello');

    await act(async () => view.result.current.handleSendDirectMessage(event));
    expect(view.store.getState().directMessageError).toBe('send failed concretely');

    await act(async () =>
      view.result.current.handleDeleteDirectMessageMessage(peerPubkey, 'message-1')
    );
    expect(view.store.getState().directMessageError).toBe('delete failed concretely');

    await act(async () => view.result.current.handleClearDirectMessage(peerPubkey));
    expect(view.store.getState().directMessageError).toBe('clear failed concretely');
  });

  test('handlePublish inserts a pending optimistic post, clears the composer, then marks it syncing after createPost resolves', async () => {
    const deferredCreatePost = createDeferred<string>();
    const createPost = vi.fn(() => deferredCreatePost.promise);
    const existingPost = buildPost({
      object_id: 'existing-post-1',
      origin_topic_id: 'kukuri:topic:demo',
    });
    const view = renderActionsHook({
      api: { createPost },
      preset: (current) => ({
        syncStatus: { ...current.syncStatus, local_author_pubkey: SELF_PUBKEY },
        // 前後空白は trim されて投稿される
        composer: '  hello world  ',
        timelinesByKey: {
          ...current.timelinesByKey,
          'kukuri:topic:demo::public': [existingPost],
        },
        publicTimelinesByTopic: {
          ...current.publicTimelinesByTopic,
          'kukuri:topic:demo': [existingPost],
        },
      }),
    });
    const { event, preventDefault } = publishFormEvent();

    await act(async () => {
      await view.result.current.handlePublish(event);
    });

    expect(preventDefault).toHaveBeenCalledTimes(1);

    // optimistic post が両タイムラインの先頭に pending で入る
    const timelinePosts = view.store.getState().timelinesByKey['kukuri:topic:demo::public'];
    expect(timelinePosts).toHaveLength(2);
    const optimistic = timelinePosts[0]!;
    expect(optimistic.object_id).toMatch(/^local-post:/);
    expect(optimistic.local_id).toBe(optimistic.object_id);
    expect(optimistic.local_state).toBe('pending');
    expect(optimistic.local_error).toBeNull();
    expect(optimistic.server_object_id).toBeNull();
    expect(optimistic.content).toBe('hello world');
    expect(optimistic.author_pubkey).toBe(SELF_PUBKEY);
    expect(optimistic.object_kind).toBe('post');
    expect(optimistic.channel_id).toBeNull();
    expect(optimistic.audience_label).toBe('Public');
    expect(optimistic.published_topic_id).toBe('kukuri:topic:demo');
    expect(optimistic.origin_topic_id).toBe('kukuri:topic:demo');
    expect(optimistic.root_id).toBe(optimistic.object_id);
    expect(optimistic.attachments).toEqual([]);
    expect(optimistic.local_draft).toEqual({
      kind: 'post',
      topic: 'kukuri:topic:demo',
      content: 'hello world',
      reply_to: null,
      channel_ref: { kind: 'public' },
      attachments: [],
    });
    expect(timelinePosts[1]?.object_id).toBe('existing-post-1');

    const publicPosts = view.store.getState().publicTimelinesByTopic['kukuri:topic:demo'];
    expect(publicPosts).toHaveLength(2);
    expect(publicPosts[0]?.object_id).toBe(optimistic.object_id);
    expect(publicPosts[0]?.local_state).toBe('pending');
    expect(publicPosts[1]?.object_id).toBe('existing-post-1');

    // composer / draft のクリアとダイアログ閉鎖
    const cleared = view.store.getState();
    expect(cleared.composer).toBe('');
    expect(cleared.draftMediaItems).toEqual([]);
    expect(cleared.attachmentInputKey).toBe(1);
    expect(cleared.composerError).toBeNull();
    expect(cleared.replyTarget).toBeNull();
    expect(cleared.repostTarget).toBeNull();
    expect(cleared.shellChromeState.activePrimarySection).toBe('timeline');
    expect(view.mocks.setComposeDialogOpen).toHaveBeenCalledTimes(1);
    expect(view.mocks.setComposeDialogOpen).toHaveBeenCalledWith(false);
    expect(view.mocks.syncRoute).toHaveBeenCalledTimes(1);
    expect(view.mocks.syncRoute).toHaveBeenCalledWith('replace', {
      primarySection: 'timeline',
    });

    // api には trim 済み本文 + public channel ref が渡る
    expect(createPost).toHaveBeenCalledTimes(1);
    expect(createPost).toHaveBeenCalledWith(
      'kukuri:topic:demo',
      'hello world',
      null,
      [],
      { kind: 'public' }
    );
    expect(view.mocks.refreshVisibleTimelineAfterPublish).not.toHaveBeenCalled();

    // createPost 解決後は syncing + server_object_id に遷移する
    await act(async () => {
      deferredCreatePost.resolve('server-object-1');
      await deferredCreatePost.promise;
    });
    await waitFor(() => {
      expect(
        view.store.getState().timelinesByKey['kukuri:topic:demo::public'][0]?.local_state
      ).toBe('syncing');
    });
    const synced = view.store.getState().timelinesByKey['kukuri:topic:demo::public'][0]!;
    expect(synced.server_object_id).toBe('server-object-1');
    expect(synced.local_error).toBeNull();
    expect(
      view.store.getState().publicTimelinesByTopic['kukuri:topic:demo'][0]?.local_state
    ).toBe('syncing');
    expect(
      view.store.getState().publicTimelinesByTopic['kukuri:topic:demo'][0]?.server_object_id
    ).toBe('server-object-1');
    expect(view.mocks.refreshVisibleTimelineAfterPublish).toHaveBeenCalledTimes(1);
    expect(view.mocks.refreshVisibleTimelineAfterPublish).toHaveBeenCalledWith(
      'kukuri:topic:demo',
      null
    );

    view.unmount();
  });

  test('handlePublish aborts a quote repost without commentary by setting composerError to the translation key', async () => {
    const createPost = vi.fn(async () => 'server-post-1');
    const createRepost = vi.fn(async () => 'server-repost-1');
    const view = renderActionsHook({
      api: { createPost, createRepost },
      preset: () => ({
        // 空白のみの本文は trim されて空扱いになる
        composer: '   ',
        repostTarget: buildPost({
          object_id: 'source-post-1',
          origin_topic_id: 'kukuri:topic:demo',
        }),
      }),
    });
    const { event } = publishFormEvent();

    await act(async () => {
      await view.result.current.handlePublish(event);
    });

    const state = view.store.getState();
    // 訳キーそのものが composerError に入る(translate stub が key を返すため)
    expect(state.composerError).toBe('common:errors.quoteRepostRequiresCommentary');
    expect(createRepost).not.toHaveBeenCalled();
    expect(createPost).not.toHaveBeenCalled();
    // 中断: composer / repostTarget は保持され、optimistic 挿入もダイアログ閉鎖も起きない
    expect(state.composer).toBe('   ');
    expect(state.repostTarget?.object_id).toBe('source-post-1');
    expect(state.timelinesByKey['kukuri:topic:demo::public']).toEqual([]);
    expect(state.publicTimelinesByTopic['kukuri:topic:demo']).toEqual([]);
    expect(view.mocks.setComposeDialogOpen).not.toHaveBeenCalled();
    expect(view.mocks.syncRoute).not.toHaveBeenCalled();

    view.unmount();
  });

  test('handlePublish marks the optimistic post failed and surfaces the error when createPost rejects', async () => {
    const createPost = vi.fn(async (): Promise<string> => {
      throw new Error('publish failed: boom');
    });
    const view = renderActionsHook({
      api: { createPost },
      preset: (current) => ({
        syncStatus: { ...current.syncStatus, local_author_pubkey: SELF_PUBKEY },
        composer: 'will fail',
      }),
    });
    const { event } = publishFormEvent();

    await act(async () => {
      await view.result.current.handlePublish(event);
    });

    await waitFor(() => {
      expect(
        view.store.getState().publicTimelinesByTopic['kukuri:topic:demo'][0]?.local_state
      ).toBe('failed');
    });
    const failedPost = view.store.getState().publicTimelinesByTopic['kukuri:topic:demo'][0]!;
    expect(failedPost.local_error).toBe('publish failed: boom');
    const timelinePost = view.store.getState().timelinesByKey['kukuri:topic:demo::public'][0]!;
    expect(timelinePost.local_state).toBe('failed');
    expect(timelinePost.local_error).toBe('publish failed: boom');
    // Error.message がそのまま composerError になる(訳キーではない)
    expect(view.store.getState().composerError).toBe('publish failed: boom');
    expect(view.mocks.refreshVisibleTimelineAfterPublish).not.toHaveBeenCalled();

    view.unmount();
  });

  test('handleRestoreLocalPost restores composer state from local_draft and reopens the compose dialog', () => {
    const parentPost = buildPost({
      object_id: 'parent-post-1',
      root_id: 'parent-root-1',
    });
    const failedPost = buildPost({
      object_id: 'local-post:failed-1',
      local_id: 'local-post:failed-1',
      object_kind: 'comment',
      root_id: 'parent-root-1',
      reply_to: 'parent-post-1',
      content: 'draft content to restore',
      local_state: 'failed',
      local_error: 'previous publish error',
      local_draft: {
        kind: 'post',
        topic: 'kukuri:topic:demo',
        content: 'draft content to restore',
        reply_to: 'parent-post-1',
        channel_ref: { kind: 'public' },
        attachments: [],
      },
      local_draft_media_items: [
        {
          id: 'draft-media-1',
          source_name: 'photo.png',
          preview_url: 'blob:draft-media-1',
          attachments: [
            {
              mime: 'image/png',
              byte_size: 128,
              data_base64: 'aGVsbG8=',
              role: 'image_original',
            },
          ],
        },
      ],
    });
    const view = renderActionsHook({
      preset: (current) => ({
        // reply_to の復元先(findKnownPost)が publicTimelinesByTopic から解決される
        publicTimelinesByTopic: {
          ...current.publicTimelinesByTopic,
          'kukuri:topic:demo': [parentPost],
        },
        shellChromeState: {
          ...current.shellChromeState,
          activePrimarySection: 'profile',
        },
      }),
    });

    act(() => {
      view.result.current.handleRestoreLocalPost(failedPost);
    });

    const state = view.store.getState();
    expect(state.composer).toBe('draft content to restore');
    expect(state.draftMediaItems).toEqual([
      {
        id: 'draft-media-1',
        source_name: 'photo.png',
        preview_url: 'blob:draft-media-1',
        attachments: [
          {
            mime: 'image/png',
            byte_size: 128,
            data_base64: 'aGVsbG8=',
            role: 'image_original',
          },
        ],
      },
    ]);
    expect(state.attachmentInputKey).toBe(1);
    expect(state.composeChannelByTopic['kukuri:topic:demo']).toEqual({ kind: 'public' });
    // replyTarget は draft.reply_to から既知 post を引き当てて復元される
    expect(state.replyTarget?.object_id).toBe('parent-post-1');
    expect(state.repostTarget).toBeNull();
    expect(state.selectedThread).toBe('parent-root-1');
    expect(state.composerError).toBe('previous publish error');
    expect(state.shellChromeState.activePrimarySection).toBe('timeline');
    expect(view.mocks.rememberDraftPreview).toHaveBeenCalledTimes(1);
    expect(view.mocks.rememberDraftPreview).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'draft-media-1' })
    );
    expect(view.mocks.setComposeDialogOpen).toHaveBeenCalledTimes(1);
    expect(view.mocks.setComposeDialogOpen).toHaveBeenCalledWith(true);
    expect(view.mocks.syncRoute).toHaveBeenCalledTimes(1);
    expect(view.mocks.syncRoute).toHaveBeenCalledWith('replace', {
      activeTopic: 'kukuri:topic:demo',
      primarySection: 'timeline',
      selectedThread: 'parent-root-1',
    });

    view.unmount();
  });

  test('handleToggleBookmarkedPost bookmarks then removes the post through the api', async () => {
    const post = buildPost({
      object_id: 'bookmark-post-1',
      origin_topic_id: 'kukuri:topic:demo',
    });
    const bookmarkedView: BookmarkedPostView = {
      bookmarked_at: 1111,
      post: buildPost({
        object_id: 'bookmark-post-1',
        origin_topic_id: 'kukuri:topic:demo',
      }),
    };
    const bookmarkPost = vi.fn(async () => bookmarkedView);
    const removeBookmarkedPost = vi.fn(async () => undefined);
    const view = renderActionsHook({
      api: { bookmarkPost, removeBookmarkedPost },
      preset: () => ({ error: 'previous error' }),
    });

    await act(async () => {
      await view.result.current.handleToggleBookmarkedPost(post);
    });

    expect(bookmarkPost).toHaveBeenCalledTimes(1);
    expect(bookmarkPost).toHaveBeenCalledWith('kukuri:topic:demo', 'bookmark-post-1');
    expect(removeBookmarkedPost).not.toHaveBeenCalled();
    expect(view.store.getState().bookmarkedPosts).toHaveLength(1);
    expect(view.store.getState().bookmarkedPosts[0]?.bookmarked_at).toBe(1111);
    expect(view.store.getState().bookmarkedPosts[0]?.post.object_id).toBe('bookmark-post-1');
    expect(view.store.getState().error).toBeNull();

    // bookmarkedPosts 更新でフックが再レンダーされ、同じ post の 2 回目は解除になる
    await act(async () => {
      await view.result.current.handleToggleBookmarkedPost(post);
    });

    expect(removeBookmarkedPost).toHaveBeenCalledTimes(1);
    expect(removeBookmarkedPost).toHaveBeenCalledWith('bookmark-post-1');
    expect(bookmarkPost).toHaveBeenCalledTimes(1);
    expect(view.store.getState().bookmarkedPosts).toEqual([]);

    view.unmount();
  });

  test('handleToggleReaction patches reaction state into timelines and refreshes recent reactions', async () => {
    const post = buildPost({
      object_id: 'reaction-post-1',
      origin_topic_id: 'kukuri:topic:demo',
    });
    const reactionState: ReactionStateView = {
      target_object_id: 'reaction-post-1',
      source_replica_id: 'replica-local-1',
      reaction_summary: [
        {
          reaction_key_kind: 'emoji',
          normalized_reaction_key: 'emoji:👍',
          emoji: '👍',
          custom_asset: null,
          count: 1,
        },
      ],
      my_reactions: [
        {
          reaction_key_kind: 'emoji',
          normalized_reaction_key: 'emoji:👍',
          emoji: '👍',
          custom_asset: null,
        },
      ],
    };
    const recentReactions: RecentReactionView[] = [
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        updated_at: 2222,
      },
    ];
    const toggleReaction = vi.fn(async () => reactionState);
    const listRecentReactions = vi.fn(async () => recentReactions);
    const view = renderActionsHook({
      api: { toggleReaction, listRecentReactions },
      preset: (current) => ({
        timelinesByKey: {
          ...current.timelinesByKey,
          'kukuri:topic:demo::public': [post],
        },
        publicTimelinesByTopic: {
          ...current.publicTimelinesByTopic,
          'kukuri:topic:demo': [post],
        },
        error: 'previous error',
      }),
    });

    await act(async () => {
      await view.result.current.handleToggleReaction(post, { kind: 'emoji', emoji: '👍' });
    });

    // channel_id 無しの post は public channel ref で toggle される
    expect(toggleReaction).toHaveBeenCalledTimes(1);
    expect(toggleReaction).toHaveBeenCalledWith(
      'kukuri:topic:demo',
      'reaction-post-1',
      { kind: 'emoji', emoji: '👍' },
      { kind: 'public' }
    );
    const state = view.store.getState();
    expect(state.timelinesByKey['kukuri:topic:demo::public'][0]?.reaction_summary).toEqual([
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        count: 1,
      },
    ]);
    expect(state.timelinesByKey['kukuri:topic:demo::public'][0]?.my_reactions).toEqual([
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
      },
    ]);
    expect(state.publicTimelinesByTopic['kukuri:topic:demo'][0]?.reaction_summary).toEqual([
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        count: 1,
      },
    ]);
    expect(listRecentReactions).toHaveBeenCalledTimes(1);
    expect(listRecentReactions).toHaveBeenCalledWith(8);
    expect(state.recentReactions).toEqual([
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        updated_at: 2222,
      },
    ]);
    expect(state.error).toBeNull();

    view.unmount();
  });

  test('handleSelectTopic activates the topic with public scope and resets thread context', async () => {
    const threadPost = buildPost({ object_id: 'thread-post-1' });
    const view = renderActionsHook({
      preset: (current) => ({
        selectedThread: 'thread-post-1',
        thread: [threadPost],
        replyTarget: threadPost,
        repostTarget: threadPost,
        selectedAuthorPubkey: AUTHOR_A_PUBKEY,
        selectedAuthorTimeline: [threadPost],
        authorError: 'previous author error',
        // 選択し直した topic の channel 選択と scope は public にリセットされる
        selectedChannelIdByTopic: {
          ...current.selectedChannelIdByTopic,
          'kukuri:topic:iroh': 'channel-x',
        },
        timelineScopeByTopic: {
          ...current.timelineScopeByTopic,
          'kukuri:topic:iroh': { kind: 'channel', channel_id: 'channel-x' },
        },
        composeChannelByTopic: {
          ...current.composeChannelByTopic,
          'kukuri:topic:iroh': { kind: 'private_channel', channel_id: 'channel-x' },
        },
        shellChromeState: {
          ...current.shellChromeState,
          activePrimarySection: 'profile',
          navOpen: true,
        },
      }),
    });

    await act(async () => {
      await view.result.current.handleSelectTopic('kukuri:topic:iroh');
    });

    const state = view.store.getState();
    expect(state.activeTopic).toBe('kukuri:topic:iroh');
    expect(state.selectedChannelIdByTopic['kukuri:topic:iroh']).toBeNull();
    expect(state.timelineScopeByTopic['kukuri:topic:iroh']).toEqual({ kind: 'public' });
    expect(state.composeChannelByTopic['kukuri:topic:iroh']).toEqual({ kind: 'public' });
    expect(state.shellChromeState.activePrimarySection).toBe('timeline');
    expect(state.shellChromeState.navOpen).toBe(false);
    // clearThreadContext 相当のリセット
    expect(state.selectedThread).toBeNull();
    expect(state.thread).toEqual([]);
    expect(state.replyTarget).toBeNull();
    expect(state.repostTarget).toBeNull();
    expect(state.selectedAuthorPubkey).toBeNull();
    expect(state.selectedAuthor).toBeNull();
    expect(state.selectedAuthorTimeline).toEqual([]);
    expect(state.authorError).toBeNull();
    expect(view.mocks.syncRoute).toHaveBeenCalledTimes(1);
    expect(view.mocks.syncRoute).toHaveBeenCalledWith('replace', {
      activeTopic: 'kukuri:topic:iroh',
      primarySection: 'timeline',
      timelineScope: { kind: 'public' },
      composeTarget: { kind: 'public' },
    });
    expect(view.mocks.loadTopics).toHaveBeenCalledTimes(1);
    expect(view.mocks.loadTopics).toHaveBeenCalledWith(
      ['kukuri:topic:demo', 'kukuri:topic:iroh', 'kukuri:topic:nostr', 'kukuri:topic:operators'],
      'kukuri:topic:iroh',
      null
    );

    view.unmount();
  });
});
