/**
 * WP-S5: routing/useRouteSynchronization の characterization テスト。
 *
 * 後続 WP-H6(shell store のスライス化・selector 化・prop drilling 除去)の
 * 安全網として「現時点で観測した挙動」をそのまま固定する。
 * このテストが落ちた場合、疑うべきは加えた変更でありテストではない。
 *
 * 方針:
 * - 全 13 依存が引数注入のため wrapper 不要。navigate / openThread / openAuthorDetail /
 *   openDirectMessagePane / syncRoute / loadTopics は vi.fn、scheduleAnimationFrame は
 *   即時実行 stub を注入する。normalize の rAF 連鎖は syncRoute / navigate を vi.fn に
 *   することで断つ(実 navigate は渡さない)。
 * - state は手渡しスナップショットであり、effect 内の setField では自動再レンダー
 *   されない。そのため「route 変化 1 回分の 1 ステップ挙動」を 1 テストで固定し、
 *   多段遷移は rerender で次のスナップショットを渡して観測する。
 * - 固定するのは注入コールバックの呼び出し引数(toHaveBeenCalledWith)と store の
 *   状態遷移(キー/フィールド単位)のみ。返り値やオブジェクト全量の snapshot・
 *   参照同一性は assert しない(H6 のリファクタで無意味に割れるため)。
 */
import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type { JoinedPrivateChannelView, PostView } from '@/lib/api';
import { useRouteSynchronization } from '@/shell/routing/useRouteSynchronization';
import { createDesktopShellStore, type DesktopShellStoreApi } from '@/shell/store';
import { resetWindowHash } from '@/shell/testSupport/renderShellHook';
import { selectShellRoutingSlice } from '@/shell/storeSelectors';
import { activeWorkspaceScope } from '@/shell/slices/workspace';

const DM_PEER_PUBKEY = 'd'.repeat(64);

type RouteSynchronizationArgs = Parameters<typeof useRouteSynchronization>[0];

/**
 * 引数一式を生成する。state は storeApi の「現時点のスナップショット」なので、
 * store をプリセットしてから呼ぶこと(プリセット後の値が effect に渡る)。
 */
function createHookArgs(
  storeApi: DesktopShellStoreApi,
  overrides: Partial<RouteSynchronizationArgs> = {}
): RouteSynchronizationArgs {
  return {
    loadTopics: vi.fn(() => Promise.resolve()),
    lastObservedRouteUrlRef: { current: '' },
    navigate: vi.fn(),
    openAuthorDetail: vi.fn(() => Promise.resolve()),
    openDirectMessagePane: vi.fn(() => Promise.resolve()),
    openThread: vi.fn(() => Promise.resolve()),
    pendingRouteUrlRef: { current: null },
    resolvedRouteLocation: { pathname: '/timeline', search: '?topic=kukuri%3Atopic%3Ademo' },
    routeSection: 'timeline',
    scheduleAnimationFrame: (callback: () => void) => callback(),
    state: selectShellRoutingSlice(storeApi.getState()),
    storeApi,
    syncRoute: vi.fn(),
    ...overrides,
  };
}

function buildPost(overrides: Partial<PostView> = {}): PostView {
  return {
    object_id: 'post-1',
    envelope_id: 'envelope-post-1',
    author_pubkey: 'a'.repeat(64),
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

function buildJoinedChannel(
  channelId: string,
  label: string,
  overrides: Partial<JoinedPrivateChannelView> = {}
): JoinedPrivateChannelView {
  return {
    topic_id: 'kukuri:topic:demo',
    channel_id: channelId,
    label,
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

describe('useRouteSynchronization', () => {
  beforeEach(() => {
    resetWindowHash();
  });

  describe('unknown pathname', () => {
    test('redirects to /timeline preserving the search via replace navigation', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: { pathname: '/channels', search: '?topic=kukuri%3Atopic%3Ademo' },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(args.navigate).toHaveBeenCalledTimes(1);
      expect(args.navigate).toHaveBeenCalledWith('/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: true,
      });
      // リダイレクト判定より前に lastNonNotificationsRoute へ現 URL が記録される(観測挙動)
      expect(storeApi.getState().lastNonNotificationsRoute).toBe(
        '/channels?topic=kukuri%3Atopic%3Ademo'
      );
      expect(args.syncRoute).not.toHaveBeenCalled();
      expect(args.loadTopics).not.toHaveBeenCalled();
      view.unmount();
    });
  });

  describe('topic param', () => {
    test('switches activeTopic and reloads when the topic param is a tracked topic', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: { pathname: '/timeline', search: '?topic=kukuri%3Atopic%3Airoh' },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(activeWorkspaceScope(storeApi.getState().workspaceState).topicId).toBe('kukuri:topic:iroh');
      expect(args.loadTopics).toHaveBeenCalledTimes(1);
      expect(args.loadTopics).toHaveBeenCalledWith(
        ['kukuri:topic:demo', 'kukuri:topic:iroh', 'kukuri:topic:nostr', 'kukuri:topic:operators'],
        'kukuri:topic:iroh',
        null
      );
      expect(args.syncRoute).not.toHaveBeenCalled();
      expect(args.navigate).not.toHaveBeenCalled();
      view.unmount();
    });

    test('passes the requested threadId to loadTopics when a tracked-topic switch combines with thread context', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Airoh&context=thread&threadId=post-9',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(activeWorkspaceScope(storeApi.getState().workspaceState).topicId).toBe('kukuri:topic:iroh');
      // topic 切替(shouldReload)× context=thread の複合では loadTopics の第 3 引数に
      // requestedThreadId がそのまま渡る(useRouteSynchronization.ts L827-833)。
      expect(args.loadTopics).toHaveBeenCalledTimes(1);
      expect(args.loadTopics).toHaveBeenCalledWith(
        ['kukuri:topic:demo', 'kukuri:topic:iroh', 'kukuri:topic:nostr', 'kukuri:topic:operators'],
        'kukuri:topic:iroh',
        'post-9'
      );
      // thread のオープン自体は openThread 側へ配線される(threadId が selectedThread と異なるため)。
      expect(args.openThread).toHaveBeenCalledTimes(1);
      expect(args.openThread).toHaveBeenCalledWith('post-9', {
        focusObjectId: null,
        historyMode: 'replace',
        normalizeOnEmpty: true,
        topic: 'kukuri:topic:iroh',
      });
      expect(args.syncRoute).not.toHaveBeenCalled();
      view.unmount();
    });

    test('normalizes an untracked topic param via syncRoute replace without changing activeTopic', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Aunknown',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      // activeTopic は現状維持のまま、URL 側だけを replace で正規化する
      expect(activeWorkspaceScope(storeApi.getState().workspaceState).topicId).toBe('kukuri:topic:demo');
      expect(args.loadTopics).not.toHaveBeenCalled();
      expect(args.navigate).not.toHaveBeenCalled();
      expect(args.syncRoute).toHaveBeenCalledTimes(1);
      expect(args.syncRoute).toHaveBeenCalledWith('replace', {
        activeTopic: 'kukuri:topic:demo',
        composeTarget: { kind: 'public' },
        focusedObjectId: null,
        primarySection: 'timeline',
        profileConnectionsView: 'following',
        profileMode: 'overview',
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedGameRoomId: null,
        selectedLiveSessionId: null,
        selectedThread: null,
        settingsOpen: false,
        settingsSection: 'connectivity',
        timelineScope: { kind: 'public' },
        timelineView: 'feed',
      });
      view.unmount();
    });
  });

  describe('channel param', () => {
    test('normalizes the channel param to null when the channel panel is ready and the channel is not joined', () => {
      const storeApi = createDesktopShellStore();
      const initial = storeApi.getState();
      storeApi.getState().patchState({
        channelPanelStateByTopic: {
          ...initial.channelPanelStateByTopic,
          'kukuri:topic:demo': { status: 'ready', error: null },
        },
      });
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Ademo&channel=chan-1',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      // この assert は「store が書き換えられない(現状維持)」ことの確認。デフォルト値も
      // null であり、current===next===null のため map は書かれない
      // (useRouteSynchronization.ts L249-266)。実質的な normalize の固定は
      // 直後の syncRoute 引数 assert が担う。
      expect(activeWorkspaceScope(storeApi.getState().workspaceState).channelId).toBeNull();
      expect(args.loadTopics).not.toHaveBeenCalled();
      expect(args.syncRoute).toHaveBeenCalledTimes(1);
      // channel は落とされ public scope / public compose target へ正規化される
      expect(args.syncRoute).toHaveBeenCalledWith('replace', {
        activeTopic: 'kukuri:topic:demo',
        composeTarget: { kind: 'public' },
        focusedObjectId: null,
        primarySection: 'timeline',
        profileConnectionsView: 'following',
        profileMode: 'overview',
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedGameRoomId: null,
        selectedLiveSessionId: null,
        selectedThread: null,
        settingsOpen: false,
        settingsSection: 'connectivity',
        timelineScope: { kind: 'public' },
        timelineView: 'feed',
      });
      view.unmount();
    });

    test('keeps the channel param pending while the channel panel is not ready yet', () => {
      // channelPanelStateByTopic はデフォルトで status='loading'(未 ready)
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Ademo&channel=chan-1',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      // pending validation 中は store も URL も書き換えない(channel は保持される)
      expect(args.syncRoute).not.toHaveBeenCalled();
      expect(args.navigate).not.toHaveBeenCalled();
      expect(args.loadTopics).not.toHaveBeenCalled();
      expect(activeWorkspaceScope(storeApi.getState().workspaceState).channelId).toBeNull();
      expect(storeApi.getState().timelineScopeByTopic['kukuri:topic:demo']).toEqual({
        kind: 'public',
      });
      view.unmount();
    });

    test('adopts a joined channel param into selection, scope and compose target then reloads', () => {
      const storeApi = createDesktopShellStore();
      const initial = storeApi.getState();
      storeApi.getState().patchState({
        channelPanelStateByTopic: {
          ...initial.channelPanelStateByTopic,
          'kukuri:topic:demo': { status: 'ready', error: null },
        },
        joinedChannelsByTopic: {
          ...initial.joinedChannelsByTopic,
          'kukuri:topic:demo': [buildJoinedChannel('chan-1', 'General')],
        },
      });
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Ademo&channel=chan-1',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      const state = storeApi.getState();
      expect(activeWorkspaceScope(state.workspaceState).channelId).toBe('chan-1');
      expect(state.timelineScopeByTopic['kukuri:topic:demo']).toEqual({
        kind: 'channel',
        channel_id: 'chan-1',
      });
      expect(state.composeChannelByTopic['kukuri:topic:demo']).toEqual({
        kind: 'private_channel',
        channel_id: 'chan-1',
      });
      expect(args.loadTopics).toHaveBeenCalledTimes(1);
      expect(args.loadTopics).toHaveBeenCalledWith(
        ['kukuri:topic:demo', 'kukuri:topic:iroh', 'kukuri:topic:nostr', 'kukuri:topic:operators'],
        'kukuri:topic:demo',
        null
      );
      expect(args.syncRoute).not.toHaveBeenCalled();
      view.unmount();
    });
  });

  describe('thread context', () => {
    test('opens the thread once when threadId differs and does not re-open after the snapshot catches up', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1',
        },
      });

      const view = renderHook(
        (props: RouteSynchronizationArgs) => useRouteSynchronization(props),
        { initialProps: args }
      );

      expect(args.openThread).toHaveBeenCalledTimes(1);
      expect(args.openThread).toHaveBeenCalledWith('post-1', {
        focusObjectId: null,
        historyMode: 'replace',
        normalizeOnEmpty: true,
        topic: 'kukuri:topic:demo',
      });
      expect(args.syncRoute).not.toHaveBeenCalled();

      // openThread 完了相当の store 更新を模し、次のスナップショットで effect を再実行する
      act(() => {
        storeApi.getState().patchState({
          selectedThread: 'post-1',
          threadsById: { 'post-1': [buildPost({ object_id: 'post-1' })] },
        });
      });
      view.rerender({ ...args, state: selectShellRoutingSlice(storeApi.getState()) });

      // threadId が selectedThread と一致し thread が読み込み済みなら再オープンしない
      expect(args.openThread).toHaveBeenCalledTimes(1);
      expect(args.syncRoute).not.toHaveBeenCalled();
      view.unmount();
    });
  });

  describe('messages peerPubkey', () => {
    test('opens the direct message pane for a hex64 peerPubkey on the messages section', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/messages',
          search: `?topic=kukuri%3Atopic%3Ademo&peerPubkey=${DM_PEER_PUBKEY}`,
        },
        routeSection: 'messages',
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(storeApi.getState().directMessagePaneOpen).toBe(true);
      expect(args.openDirectMessagePane).toHaveBeenCalledTimes(1);
      expect(args.openDirectMessagePane).toHaveBeenCalledWith(DM_PEER_PUBKEY, {
        historyMode: 'replace',
        normalizeOnError: true,
        preserveAuthorPane: false,
        preservedAuthorPubkey: null,
      });
      expect(args.syncRoute).not.toHaveBeenCalled();
      expect(args.navigate).not.toHaveBeenCalled();
      view.unmount();
    });

    test('normalizes a non-hex64 peerPubkey to null without opening a conversation', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        resolvedRouteLocation: {
          pathname: '/messages',
          search: '?topic=kukuri%3Atopic%3Ademo&peerPubkey=not-a-key',
        },
        routeSection: 'messages',
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(args.openDirectMessagePane).not.toHaveBeenCalled();
      // ペイン自体は開かれる(観測挙動)が、peer は URL から落とされる
      expect(storeApi.getState().directMessagePaneOpen).toBe(true);
      expect(args.syncRoute).toHaveBeenCalledTimes(1);
      expect(args.syncRoute).toHaveBeenCalledWith('replace', {
        activeTopic: 'kukuri:topic:demo',
        composeTarget: { kind: 'public' },
        focusedObjectId: null,
        primarySection: 'messages',
        profileConnectionsView: 'following',
        profileMode: 'overview',
        selectedAuthorPubkey: null,
        selectedDirectMessagePeerPubkey: null,
        selectedGameRoomId: null,
        selectedLiveSessionId: null,
        selectedThread: null,
        settingsOpen: false,
        settingsSection: 'connectivity',
        timelineScope: { kind: 'public' },
        timelineView: 'feed',
      });
      view.unmount();
    });
  });

  describe('route observation gate', () => {
    test('skips processing while a pending route url has not been observed and the route did not change', () => {
      const storeApi = createDesktopShellStore();
      // 未追跡 topic(本来なら normalize される URL)でも、pending 未観測なら何もしない
      const args = createHookArgs(storeApi, {
        lastObservedRouteUrlRef: { current: '/timeline?topic=kukuri%3Atopic%3Aunknown' },
        pendingRouteUrlRef: { current: '/timeline?topic=kukuri%3Atopic%3Ademo' },
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Aunknown',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(args.syncRoute).not.toHaveBeenCalled();
      expect(args.navigate).not.toHaveBeenCalled();
      expect(args.loadTopics).not.toHaveBeenCalled();
      expect(args.pendingRouteUrlRef.current).toBe('/timeline?topic=kukuri%3Atopic%3Ademo');
      expect(storeApi.getState().lastNonNotificationsRoute).toBeNull();
      view.unmount();
    });

    test('clears the pending route url and resumes processing when the route changed', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        lastObservedRouteUrlRef: { current: '/timeline?topic=kukuri%3Atopic%3Ademo' },
        pendingRouteUrlRef: { current: '/timeline?topic=kukuri%3Atopic%3Ademo' },
        resolvedRouteLocation: {
          pathname: '/timeline',
          search: '?topic=kukuri%3Atopic%3Aunknown',
        },
      });

      const view = renderHook(() => useRouteSynchronization(args));

      expect(args.pendingRouteUrlRef.current).toBeNull();
      expect(args.lastObservedRouteUrlRef.current).toBe('/timeline?topic=kukuri%3Atopic%3Aunknown');
      expect(storeApi.getState().lastNonNotificationsRoute).toBe(
        '/timeline?topic=kukuri%3Atopic%3Aunknown'
      );
      // 未追跡 topic の normalize が通常どおり走る
      expect(args.syncRoute).toHaveBeenCalledTimes(1);
      view.unmount();
    });
  });
});
