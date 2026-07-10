/**
 * WP-S5: useDesktopShellData の characterization テスト。
 *
 * 後続 WP-H6(shell store のスライス化・selector 化・prop drilling 除去)の
 * 安全網として「現時点で観測した挙動」をそのまま固定する。
 * このテストが落ちた場合、疑うべきは加えた変更でありテストではない。
 *
 * 方針:
 * - フックは mount 直後に refresh / getMyProfile / 通知ポーリング(10s)/
 *   タイムライン refresh(3s 間隔)を無条件に開始するため、renderHook の前に
 *   vi.useFakeTimers() を必ず有効化し、api は createDesktopMockApi を使う。
 * - mount 直後の非同期 flush は `await act(async () => vi.advanceTimersByTimeAsync(0))`
 *   (前例: DesktopShellPage.timelineRefresh.test.tsx)。interval 起動は
 *   REFRESH_INTERVAL_MS の advance で行う(タイマー advance 量のみ定数 import 可)。
 * - 期待値は観測した現挙動の生リテラル(limit 20 / storage key 文字列 等)。
 *   translate は key をそのまま返す stub を注入し locale リソースから切り離す。
 * - 全量 snapshot・参照同一性 assert は書かない。固定するのは api 呼び出し引数
 *   (toHaveBeenCalledWith)・store の状態遷移・キー/フィールド単位の値のみ。
 * - interval リーク防止のため各テスト末尾で必ず unmount する。
 */
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

const { listenMock } = vi.hoisted(() => ({ listenMock: vi.fn() }));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => listenMock(...args),
}));

import type {
  DesktopApi,
  JoinedPrivateChannelView,
  NotificationView,
  PostView,
  TimelineCursor,
  TimelineScope,
  TimelineView,
} from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  CONNECTIVITY_STATUS_FALLBACK_INTERVAL_MS,
  REFRESH_INTERVAL_MS,
} from '@/shell/store';
import { useDesktopShellData } from '@/shell/useDesktopShellData';
import {
  actPatchState,
  createShellHookHarness,
  resetWindowHash,
  type ShellHookHarness,
} from '@/shell/testSupport/renderShellHook';

const AUTHOR_PUBKEY = 'a'.repeat(64);

// key をそのまま返す stub。エラーメッセージ assert を locale リソースから切り離す。
const stubTranslate = (key: string) => key;

function buildPost(overrides: Partial<PostView> = {}): PostView {
  // 添付なしを基本形にする(添付があると mount 時に getBlobMediaPayload の
  // media fetch が走るため、characterization の対象外ノイズになる)。
  return {
    object_id: 'post-1',
    envelope_id: 'envelope-post-1',
    author_pubkey: AUTHOR_PUBKEY,
    author_name: 'alice',
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

function buildJoinedChannel(
  channelId: string,
  overrides: Partial<JoinedPrivateChannelView> = {}
): JoinedPrivateChannelView {
  return {
    topic_id: 'kukuri:topic:demo',
    channel_id: channelId,
    label: 'core',
    creator_pubkey: 'c'.repeat(64),
    owner_pubkey: 'c'.repeat(64),
    joined_via_pubkey: null,
    audience_kind: 'invite_only',
    is_owner: false,
    current_epoch_id: 'epoch-1',
    archived_epoch_ids: [],
    sharing_state: 'open',
    rotation_required: false,
    participant_count: 2,
    stale_participant_count: 0,
    ...overrides,
  };
}

function buildNotification(overrides: Partial<NotificationView> = {}): NotificationView {
  return {
    notification_id: 'notification-1',
    kind: 'reply',
    actor_pubkey: 'c'.repeat(64),
    actor_name: 'carol',
    actor_display_name: null,
    actor_picture: null,
    actor_picture_asset: null,
    source_envelope_id: 'notification-envelope-1',
    source_replica_id: 'replica:notification',
    topic_id: 'kukuri:topic:demo',
    channel_id: null,
    object_id: 'reply-1',
    thread_root_object_id: 'post-1',
    dm_id: null,
    message_id: null,
    preview_text: 'notification preview',
    created_at: 1,
    received_at: 1,
    read_at: null,
    ...overrides,
  };
}

// DesktopShellPage.testHelpers と同型の deferred(App 依存の module を
// unit 並列 lane に持ち込まないためローカル定義)。
function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return { promise, resolve, reject };
}

/**
 * useDesktopShellData を renderHook でマウントする。
 * ref 6 本は 1 テスト 1 回だけ生成して安定させる(毎レンダー再生成すると
 * refreshVisibleShellData の useCallback が無効化され refresh effect が再発火するため)。
 */
function renderDataHook(
  api: DesktopApi,
  harness: ShellHookHarness = createShellHookHarness()
) {
  const refs = {
    loadTopicsRequestRef: { current: 0 },
    remoteObjectUrlRef: { current: new Map<string, string>() },
    draftPreviewUrlRef: { current: new Map<string, string>() },
    directMessageDraftPreviewUrlRef: { current: new Map<string, string>() },
    mediaFetchAttemptRef: { current: new Map<string, number>() },
    draftSequenceRef: { current: 0 },
  };
  const view = renderHook(
    () => useDesktopShellData({ api, translate: stubTranslate, ...refs }),
    { wrapper: harness.wrapper }
  );
  return { harness, view };
}

/** mount 直後の初回 fetch(microtask 連鎖)を act 内で flush する。 */
async function flushAsyncWork(): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(0);
  });
}

/** fake timer を ms だけ進め、発火した interval の非同期連鎖まで flush する。 */
async function advanceTimersAsync(ms: number): Promise<void> {
  await act(async () => {
    await vi.advanceTimersByTimeAsync(ms);
  });
}

beforeEach(() => {
  // setup.ts は hash を掃除しないため毎テスト '/' へ戻す。
  resetWindowHash();
  // mount 直後からポーリングが走るため renderHook 前に必ず fake timers。
  // (afterEach の useRealTimers は setup.ts が行う)
  vi.useFakeTimers();
  listenMock.mockReset();
  listenMock.mockResolvedValue(() => undefined);
  delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
});

describe('useDesktopShellData characterization', () => {
  test('mount refresh calls listTimeline for both active and public scopes with limit 20 and fills the store', async () => {
    const channelPost = buildPost({
      object_id: 'post-channel',
      root_id: 'post-channel',
      channel_id: 'channel-1',
      audience_label: 'core',
      created_at: 2,
    });
    const publicPost = buildPost({
      object_id: 'post-public',
      root_id: 'post-public',
      created_at: 1,
    });
    const joinedChannel = buildJoinedChannel('channel-1');
    const baseApi = createDesktopMockApi();
    const listTimeline = vi.fn(
      async (
        _topic: string,
        _cursor?: TimelineCursor | null,
        _limit?: number,
        scope?: TimelineScope
      ): Promise<TimelineView> => {
        if (scope?.kind === 'channel') {
          return { items: [channelPost], next_cursor: null };
        }
        return { items: [publicPost], next_cursor: null };
      }
    );
    // 参加済み channel を返し続けないと dataEffects が selectedChannelId を
    // null へ戻してしまうため、listJoinedPrivateChannels も固定する。
    const listJoinedPrivateChannels = vi.fn(async () => [joinedChannel]);
    const api: DesktopApi = { ...baseApi, listTimeline, listJoinedPrivateChannels };

    // channel 選択中にすると「アクティブ scope(channel)+ public scope」の
    // 2 本が別 scope で観測できる(未選択だと両方 {kind:'public'} になる)。
    const harness = createShellHookHarness();
    harness.store.getState().patchState({
      joinedChannelsByTopic: {
        ...harness.store.getState().joinedChannelsByTopic,
        'kukuri:topic:demo': [joinedChannel],
      },
      selectedChannelIdByTopic: {
        ...harness.store.getState().selectedChannelIdByTopic,
        'kukuri:topic:demo': 'channel-1',
      },
    });

    const { view } = renderDataHook(api, harness);
    await flushAsyncWork();

    // mount 直後の refresh 1 回分: アクティブ scope + public scope の 2 呼び出しのみ。
    expect(listTimeline).toHaveBeenCalledTimes(2);
    expect(listTimeline).toHaveBeenNthCalledWith(1, 'kukuri:topic:demo', null, 20, {
      kind: 'channel',
      channel_id: 'channel-1',
    });
    expect(listTimeline).toHaveBeenNthCalledWith(2, 'kukuri:topic:demo', null, 20, {
      kind: 'public',
    });

    const state = harness.store.getState();
    expect(
      state.timelinesByKey['kukuri:topic:demo::channel::channel-1']?.map(
        (post) => post.object_id
      )
    ).toEqual(['post-channel']);
    expect(
      state.publicTimelinesByTopic['kukuri:topic:demo']?.map((post) => post.object_id)
    ).toEqual(['post-public']);
    expect(state.timelineNextCursorByKey['kukuri:topic:demo::channel::channel-1']).toBeNull();

    view.unmount();
  });

  test('background refresh buffers new authoritative posts as pending without touching the visible timeline', async () => {
    const olderPost = buildPost({
      object_id: 'post-old',
      root_id: 'post-old',
      created_at: 1,
      content: 'older post',
    });
    const newerPost = buildPost({
      object_id: 'post-new',
      root_id: 'post-new',
      created_at: 2,
      content: 'newer post',
    });
    let timelineItems = [olderPost];
    const baseApi = createDesktopMockApi();
    const listTimeline = vi.fn(
      async (): Promise<TimelineView> => ({ items: [...timelineItems], next_cursor: null })
    );
    const api: DesktopApi = { ...baseApi, listTimeline };

    const { harness, view } = renderDataHook(api);
    await flushAsyncWork();

    // 前提: 初回 refresh は baseline が空(authoritative 無し)のため直接反映される。
    expect(
      harness.store.getState().timelinesByKey['kukuri:topic:demo::public']?.map(
        (post) => post.object_id
      )
    ).toEqual(['post-old']);

    // authoritative なベースラインがある状態で新着が届くと buffer される。
    timelineItems = [newerPost, olderPost];
    await advanceTimersAsync(REFRESH_INTERVAL_MS);

    const state = harness.store.getState();
    // 可視タイムラインは不変。
    expect(
      state.timelinesByKey['kukuri:topic:demo::public']?.map((post) => post.object_id)
    ).toEqual(['post-old']);
    // 新着は pending 3 フィールドへ退避される(snapshot は最新ページ全量)。
    expect(state.pendingTimelineCountsByKey['kukuri:topic:demo::public']).toBe(1);
    expect(
      state.pendingTimelineSnapshotsByKey['kukuri:topic:demo::public']?.map(
        (post) => post.object_id
      )
    ).toEqual(['post-new', 'post-old']);
    expect('kukuri:topic:demo::public' in state.pendingTimelineNextCursorByKey).toBe(true);
    expect(state.pendingTimelineNextCursorByKey['kukuri:topic:demo::public']).toBeNull();
    // public timeline には pending バッファが無く、そのまま merge される。
    expect(
      state.publicTimelinesByTopic['kukuri:topic:demo']?.map((post) => post.object_id)
    ).toEqual(['post-new', 'post-old']);

    view.unmount();
  });

  test('connectivity status uses mount bootstrap and 60 second fallback, not the 3 second refresh', async () => {
    (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__ = {};
    const baseApi = createDesktopMockApi();
    const getSyncStatus = vi.fn(baseApi.getSyncStatus);
    const getCommunityNodeStatuses = vi.fn(baseApi.getCommunityNodeStatuses);
    const api: DesktopApi = {
      ...baseApi,
      getSyncStatus,
      getCommunityNodeStatuses,
    };

    const { view } = renderDataHook(api);
    await flushAsyncWork();
    expect(getSyncStatus).toHaveBeenCalledTimes(1);
    expect(getCommunityNodeStatuses).toHaveBeenCalledTimes(1);

    getSyncStatus.mockClear();
    getCommunityNodeStatuses.mockClear();
    await advanceTimersAsync(REFRESH_INTERVAL_MS);
    expect(getSyncStatus).not.toHaveBeenCalled();
    expect(getCommunityNodeStatuses).not.toHaveBeenCalled();

    act(() => window.dispatchEvent(new Event('focus')));
    await flushAsyncWork();
    expect(getSyncStatus).not.toHaveBeenCalled();
    expect(getCommunityNodeStatuses).not.toHaveBeenCalled();

    await advanceTimersAsync(
      CONNECTIVITY_STATUS_FALLBACK_INTERVAL_MS - REFRESH_INTERVAL_MS
    );
    expect(getSyncStatus).toHaveBeenCalledTimes(1);
    expect(getCommunityNodeStatuses).toHaveBeenCalledTimes(1);

    view.unmount();
    delete (window as unknown as Record<string, unknown>).__TAURI_INTERNALS__;
  });

  test('refreshTimelineFeed applies pending posts without api calls and clears the pending buffers', async () => {
    const olderPost = buildPost({
      object_id: 'post-old',
      root_id: 'post-old',
      created_at: 1,
    });
    const newerPost = buildPost({
      object_id: 'post-new',
      root_id: 'post-new',
      created_at: 2,
    });
    const baseApi = createDesktopMockApi();
    const listTimeline = vi.fn(
      async (): Promise<TimelineView> => ({ items: [olderPost], next_cursor: null })
    );
    const getSyncStatus = vi.fn(baseApi.getSyncStatus);
    const api: DesktopApi = { ...baseApi, listTimeline, getSyncStatus };

    const { harness, view } = renderDataHook(api);
    await flushAsyncWork();

    // pending 3 フィールドをプリセット(buffer 済み状態を直接再現する)。
    actPatchState(harness.store, {
      pendingTimelineSnapshotsByKey: {
        'kukuri:topic:demo::public': [newerPost, olderPost],
      },
      pendingTimelineCountsByKey: { 'kukuri:topic:demo::public': 1 },
      pendingTimelineNextCursorByKey: {
        'kukuri:topic:demo::public': { created_at: 1, object_id: 'post-old' },
      },
    });

    listTimeline.mockClear();
    getSyncStatus.mockClear();

    await act(async () => {
      await view.result.current.refreshTimelineFeed('kukuri:topic:demo', null);
    });

    // pending がある場合は api を一切呼ばずローカル merge のみで完結する。
    expect(listTimeline).not.toHaveBeenCalled();
    expect(getSyncStatus).not.toHaveBeenCalled();

    const state = harness.store.getState();
    expect(
      state.timelinesByKey['kukuri:topic:demo::public']?.map((post) => post.object_id)
    ).toEqual(['post-new', 'post-old']);
    // cursor は pendingTimelineNextCursorByKey の値へ差し替わる。
    expect(state.timelineNextCursorByKey['kukuri:topic:demo::public']).toEqual({
      created_at: 1,
      object_id: 'post-old',
    });
    // pending 3 フィールドはキーごと削除される。
    expect('kukuri:topic:demo::public' in state.pendingTimelineSnapshotsByKey).toBe(false);
    expect('kukuri:topic:demo::public' in state.pendingTimelineCountsByKey).toBe(false);
    expect('kukuri:topic:demo::public' in state.pendingTimelineNextCursorByKey).toBe(false);

    view.unmount();
  });

  test('loadMoreTimeline pages with the stored cursor, toggles the loading flag, and no-ops without a cursor', async () => {
    const page1Post = buildPost({
      object_id: 'post-page-1',
      root_id: 'post-page-1',
      created_at: 10,
    });
    const page2Post = buildPost({
      object_id: 'post-page-2',
      root_id: 'post-page-2',
      created_at: 9,
    });
    const loadMoreDeferred = createDeferred<TimelineView>();
    const baseApi = createDesktopMockApi();
    const listTimeline = vi.fn(
      (_topic: string, cursor?: TimelineCursor | null): Promise<TimelineView> => {
        if (cursor) {
          return loadMoreDeferred.promise;
        }
        return Promise.resolve({
          items: [page1Post],
          next_cursor: { created_at: 10, object_id: 'post-page-1' },
        });
      }
    );
    const api: DesktopApi = { ...baseApi, listTimeline };

    const { harness, view } = renderDataHook(api);
    await flushAsyncWork();

    // 前提: 初回 refresh で 1 ページ目と next_cursor が入っている。
    expect(harness.store.getState().timelineNextCursorByKey['kukuri:topic:demo::public']).toEqual(
      { created_at: 10, object_id: 'post-page-1' }
    );

    let loadMorePromise: Promise<void> | undefined;
    act(() => {
      loadMorePromise = view.result.current.loadMoreTimeline('kukuri:topic:demo');
    });

    // 取得中は loading フラグが true になる。
    expect(
      harness.store.getState().timelineLoadingMoreByKey['kukuri:topic:demo::public']
    ).toBe(true);
    // cursor 付きで同じ scope に対して追加ページを要求する。
    expect(listTimeline).toHaveBeenCalledTimes(3);
    expect(listTimeline).toHaveBeenLastCalledWith(
      'kukuri:topic:demo',
      { created_at: 10, object_id: 'post-page-1' },
      20,
      { kind: 'public' }
    );

    loadMoreDeferred.resolve({ items: [page2Post], next_cursor: null });
    await act(async () => {
      await loadMorePromise;
    });

    const state = harness.store.getState();
    // 2 ページ目は末尾へ追記される。
    expect(
      state.timelinesByKey['kukuri:topic:demo::public']?.map((post) => post.object_id)
    ).toEqual(['post-page-1', 'post-page-2']);
    expect(state.timelineNextCursorByKey['kukuri:topic:demo::public']).toBeNull();
    expect(state.timelineLoadingMoreByKey['kukuri:topic:demo::public']).toBe(false);

    // cursor が無い(null)場合は api を呼ばない no-op。
    await act(async () => {
      await view.result.current.loadMoreTimeline('kukuri:topic:demo');
    });
    expect(listTimeline).toHaveBeenCalledTimes(3);
    expect(
      harness.store.getState().timelineLoadingMoreByKey['kukuri:topic:demo::public']
    ).toBe(false);

    view.unmount();
  });

  test('notifications section marks unread notifications as read on mount and keeps them read via loadTopics', async () => {
    const readNotification = buildNotification({
      notification_id: 'notification-read',
      read_at: 111,
    });
    const unreadNotification = buildNotification({
      notification_id: 'notification-unread',
      object_id: 'reply-2',
      read_at: null,
    });
    const baseApi = createDesktopMockApi({
      notifications: [readNotification, unreadNotification],
    });
    const listNotifications = vi.fn(baseApi.listNotifications);
    const markAllNotificationsRead = vi.fn(baseApi.markAllNotificationsRead);
    const api: DesktopApi = { ...baseApi, listNotifications, markAllNotificationsRead };

    const harness = createShellHookHarness();
    harness.store.getState().patchState({
      shellChromeState: {
        ...harness.store.getState().shellChromeState,
        activePrimarySection: 'notifications',
      },
    });

    const { view } = renderDataHook(api, harness);
    await flushAsyncWork();

    // mount 時: 一覧取得 + 未読があるので markAllNotificationsRead が 1 回呼ばれる。
    expect(listNotifications).toHaveBeenCalledTimes(1);
    expect(markAllNotificationsRead).toHaveBeenCalledTimes(1);

    let state = harness.store.getState();
    expect(state.notifications.map((notification) => notification.notification_id)).toEqual([
      'notification-read',
      'notification-unread',
    ]);
    // 既読分は元の read_at を保持し、未読分は read_at が数値で補完される。
    expect(state.notifications[0]?.read_at).toBe(111);
    expect(state.notifications[1]?.read_at).toEqual(expect.any(Number));
    expect(state.notificationStatus).toEqual({ unread_count: 0 });
    expect(state.notificationPanelState).toEqual({ status: 'ready', error: null });
    expect(state.notificationAutoReadError).toBeNull();

    // loadTopics 経由(runLoadTopics の notifications 分岐)でも一覧を再取得するが、
    // 全件既読なら markAllNotificationsRead は追加で呼ばれない。
    await act(async () => {
      await view.result.current.loadTopics(['kukuri:topic:demo'], 'kukuri:topic:demo', null);
    });
    expect(listNotifications).toHaveBeenCalledTimes(2);
    expect(markAllNotificationsRead).toHaveBeenCalledTimes(1);

    state = harness.store.getState();
    expect(state.notificationStatus).toEqual({ unread_count: 0 });
    expect(state.notifications[0]?.read_at).toBe(111);
    expect(state.notifications[1]?.read_at).toEqual(expect.any(Number));

    view.unmount();
  });

  test('a core fetch failure stores the error message and the next successful refresh clears it', async () => {
    let failTimeline = true;
    const baseApi = createDesktopMockApi();
    const listTimeline = vi.fn(async (): Promise<TimelineView> => {
      if (failTimeline) {
        throw new Error('timeline fetch failed');
      }
      return { items: [], next_cursor: null };
    });
    const api: DesktopApi = { ...baseApi, listTimeline };

    const { harness, view } = renderDataHook(api);
    await flushAsyncWork();

    // Error インスタンスで reject された場合は message がそのまま error に入る。
    expect(harness.store.getState().error).toBe('timeline fetch failed');

    // 次の refresh が成功すると error は null に戻る。
    failTimeline = false;
    await advanceTimersAsync(REFRESH_INTERVAL_MS);
    expect(harness.store.getState().error).toBeNull();

    view.unmount();
  });

  test('a non-Error core failure falls back to the failedToLoadTopic message key', async () => {
    const baseApi = createDesktopMockApi();
    const rejection = createDeferred<TimelineView>();
    const listTimeline = vi.fn((): Promise<TimelineView> => rejection.promise);
    const api: DesktopApi = { ...baseApi, listTimeline };
    // Error でない値で reject するとフォールバック文言(translate の key)になる。
    rejection.reject('rpc failure without error instance');

    const { harness, view } = renderDataHook(api);
    await flushAsyncWork();

    expect(harness.store.getState().error).toBe('common:errors.failedToLoadTopic');

    view.unmount();
  });
});
