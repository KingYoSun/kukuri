/**
 * WP-S5: useDesktopShellRouting の characterization テスト。
 *
 * 後続 WP-H6(shell store のスライス化・selector 化・prop drilling 除去)の
 * 安全網として「現時点で観測した挙動」をそのまま固定する。
 * このテストが落ちた場合、疑うべきは加えた変更でありテストではない。
 *
 * 方針:
 * - Provider + HashRouter の実 wrapper でマウントする(MemoryRouter は不可 —
 *   resolveHashBackedRouteLocation が window.location.hash を直読するため URL と乖離する)。
 * - renderHook 前に hash を正規形(buildShellUrl が生成する形)へ設定し、内包する
 *   useRouteSynchronization の normalize が発火しない安定状態から始める。
 * - rAF はフォーカス移動と normalize 遅延にのみ使われるため microtask 実行の stub に
 *   差し替える(同期実行 stub は scheduleAnimationFrame 内の frameId 参照が
 *   const 初期化中の TDZ となり例外になるため不可)。
 * - 期待値は観測した生リテラル。api 呼び出し引数・store 状態遷移・hash・
 *   キー/フィールド単位の値のみ固定し、返り値の全量比較・snapshot・参照同一性は
 *   assert しない(H6 耐性)。
 */
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import type {
  AuthorSocialView,
  DesktopApi,
  DirectMessageConversationView,
  PostView,
} from '@/lib/api';
import type { PrimarySection } from '@/components/shell/types';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useDesktopShellRouting } from '@/shell/useDesktopShellRouting';
import {
  createShellHookHarness,
  resetWindowHash,
  type ShellHookHarness,
} from '@/shell/testSupport/renderShellHook';

const AUTHOR_PUBKEY = 'a'.repeat(64);
const DM_PEER_PUBKEY = 'd'.repeat(64);
const OTHER_PEER_PUBKEY = 'b'.repeat(64);
const STRANGER_PUBKEY = 'e'.repeat(64);
// buildShellUrl(URLSearchParams)が生成する正規形。':' は %3A にエンコードされる。
const BASE_TIMELINE_HASH = '#/timeline?topic=kukuri%3Atopic%3Ademo';

// key をそのまま返す stub。文言 assert を locale リソースから切り離す。
const stubTranslate = (key: string) => key;

function buildPost(overrides: Partial<PostView> = {}): PostView {
  return {
    object_id: 'post-1',
    envelope_id: 'envelope-post-1',
    author_pubkey: AUTHOR_PUBKEY,
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

function buildAuthor(pubkey: string, overrides: Partial<AuthorSocialView> = {}): AuthorSocialView {
  return {
    author_pubkey: pubkey,
    name: null,
    display_name: null,
    about: null,
    picture: null,
    picture_asset: null,
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    muted: false,
    ...overrides,
  };
}

function buildConversation(peerPubkey: string): DirectMessageConversationView {
  return {
    dm_id: `dm:${peerPubkey}`,
    peer_pubkey: peerPubkey,
    peer_name: null,
    peer_display_name: null,
    peer_picture: null,
    peer_picture_asset: null,
    updated_at: 0,
    last_message_at: null,
    last_message_id: null,
    last_message_preview: null,
    status: {
      peer_pubkey: peerPubkey,
      dm_id: `dm:${peerPubkey}`,
      mutual: true,
      send_enabled: true,
      peer_count: 1,
      pending_outbox_count: 0,
    },
  };
}

type RenderRoutingHookOptions = {
  /** renderHook 前に window.location.hash に反映される正規ルート('#/...' 形式)。 */
  hash: string;
  api?: DesktopApi;
  /** store 生成後・renderHook 前に呼ばれるプリセット(render 前なので act 不要)。 */
  preset?: (store: ShellHookHarness['store']) => void;
};

/**
 * useDesktopShellRouting を Provider + HashRouter 配下で単体マウントする。
 * ref 5 本は DesktopShellPage.tsx(L257-268)の useRef 初期値と同じ
 * { current: ... } リテラルで注入する。
 */
function renderRoutingHook(options: RenderRoutingHookOptions) {
  const harness = createShellHookHarness({ hash: options.hash, router: true });
  options.preset?.(harness.store);
  const api = options.api ?? createDesktopMockApi();
  const loadTopics = vi.fn(async () => {});
  const primarySectionRefs: { current: Partial<Record<PrimarySection, HTMLElement | null>> } = {
    current: {},
  };
  const navTriggerRef = { current: null as HTMLButtonElement | null };
  const settingsTriggerRef = { current: null as HTMLButtonElement | null };
  const pendingRouteUrlRef = { current: null as string | null };
  const didSyncRouteSectionRef = { current: false };
  const view = renderHook(
    () =>
      useDesktopShellRouting({
        api,
        translate: stubTranslate,
        loadTopics,
        primarySectionRefs,
        navTriggerRef,
        settingsTriggerRef,
        pendingRouteUrlRef,
        didSyncRouteSectionRef,
      }),
    { wrapper: harness.wrapper }
  );
  return { api, harness, loadTopics, view };
}

describe('useDesktopShellRouting', () => {
  beforeEach(() => {
    // setup.ts は hash を掃除しないため、各テストで '/' に戻す。
    resetWindowHash();
    // rAF を microtask 実行に差し替える(act / waitFor の await で必ず flush される)。
    // cancelAnimationFrame は no-op(残った callback はフォーカス移動のみで無害)。
    let rafSequence = 0;
    vi.spyOn(window, 'requestAnimationFrame').mockImplementation((callback) => {
      rafSequence += 1;
      const frameId = rafSequence;
      queueMicrotask(() => callback(frameId));
      return frameId;
    });
    vi.spyOn(window, 'cancelAnimationFrame').mockImplementation(() => {});
  });

  test('openThread success loads thread state, clears author/DM panes, and pushes a thread URL', async () => {
    const baseApi = createDesktopMockApi({
      seedPosts: {
        'kukuri:topic:demo': [
          buildPost({ object_id: 'post-1', root_id: 'post-1' }),
          buildPost({
            object_id: 'comment-1',
            root_id: 'post-1',
            reply_to: 'post-1',
            object_kind: 'comment',
            created_at: 2,
          }),
        ],
      },
    });
    const api = { ...baseApi, listThread: vi.fn(baseApi.listThread) };
    // URL と整合する author ペイン(+DM ペイン)が開いた状態から始めて
    // 「openThread が他ペインを閉じる」ことを観測する。nav も開いておき、
    // openThread が nav を閉じる(navOpen: false を設定する)ことも観測する。
    const { harness, loadTopics, view } = renderRoutingHook({
      hash: `#/timeline?topic=kukuri%3Atopic%3Ademo&context=author&authorPubkey=${AUTHOR_PUBKEY}`,
      api,
      preset: (store) => {
        store.getState().patchState({
          selectedAuthorPubkey: AUTHOR_PUBKEY,
          selectedAuthor: buildAuthor(AUTHOR_PUBKEY),
          directMessagePaneOpen: true,
          selectedDirectMessagePeerPubkey: DM_PEER_PUBKEY,
        });
        store.getState().patchState({
          shellChromeState: { ...store.getState().shellChromeState, navOpen: true },
        });
      },
    });

    const historyLengthBefore = window.history.length;
    await act(async () => {
      await view.result.current.openThread('post-1');
    });

    // THREAD_TIMELINE_LIMIT(=30)・cursor null で 1 回だけ取得する。
    expect(api.listThread).toHaveBeenCalledTimes(1);
    expect(api.listThread).toHaveBeenCalledWith('kukuri:topic:demo', 'post-1', null, 30);

    await waitFor(() => {
      expect(harness.store.getState().selectedThread).toBe('post-1');
    });
    const state = harness.store.getState();
    expect(state.thread.map((item) => item.object_id)).toEqual(['post-1', 'comment-1']);
    expect(state.threadNextCursorById).toEqual({ 'post-1': null });
    expect(state.focusedObjectId).toBeNull();
    // author / DM ペインは閉じる。
    expect(state.selectedAuthorPubkey).toBeNull();
    expect(state.selectedAuthor).toBeNull();
    expect(state.directMessagePaneOpen).toBe(false);
    expect(state.selectedDirectMessagePeerPubkey).toBeNull();
    expect(state.shellChromeState.activePrimarySection).toBe('timeline');
    expect(state.shellChromeState.timelineView).toBe('feed');
    // preset で navOpen=true にしてある → openThread が nav を閉じる。
    expect(state.shellChromeState.navOpen).toBe(false);
    expect(state.error).toBeNull();
    await waitFor(() => {
      expect(window.location.hash).toBe(
        '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1'
      );
    });
    // historyMode 既定は push(履歴が 1 つ積まれる)。
    expect(window.history.length).toBe(historyLengthBefore + 1);
    expect(loadTopics).not.toHaveBeenCalled();
    view.unmount();
  });

  test('openThread with normalizeOnEmpty clears all panes and replaces to a thread-less URL', async () => {
    const baseApi = createDesktopMockApi({
      seedPosts: {
        'kukuri:topic:demo': [buildPost({ object_id: 'post-1', root_id: 'post-1' })],
      },
    });
    const api = { ...baseApi, listThread: vi.fn(baseApi.listThread) };
    // thread ペインが開いた状態(URL とも整合)から、存在しない thread を開く。
    // live/game の選択も持たせ、normalizeOnEmpty がクリアすることを観測する
    // (timeline+thread route の mount 同期は live/game 選択を触らないため preset が残る)。
    const { harness, view } = renderRoutingHook({
      hash: '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1',
      api,
      preset: (store) => {
        store.getState().patchState({
          selectedThread: 'post-1',
          thread: [buildPost({ object_id: 'post-1', root_id: 'post-1' })],
          selectedLiveSessionId: 'live-1',
          selectedGameRoomId: 'room-1',
        });
      },
    });

    const historyLengthBefore = window.history.length;
    await act(async () => {
      await view.result.current.openThread('post-gone', { normalizeOnEmpty: true });
    });

    expect(api.listThread).toHaveBeenCalledWith('kukuri:topic:demo', 'post-gone', null, 30);
    await waitFor(() => {
      expect(harness.store.getState().selectedThread).toBeNull();
    });
    const state = harness.store.getState();
    expect(state.thread).toEqual([]);
    expect(state.focusedObjectId).toBeNull();
    expect(state.selectedAuthorPubkey).toBeNull();
    expect(state.selectedAuthor).toBeNull();
    expect(state.directMessagePaneOpen).toBe(false);
    expect(state.selectedDirectMessagePeerPubkey).toBeNull();
    // preset で非 null にしてある live/game の選択も normalizeOnEmpty がクリアする。
    expect(state.selectedLiveSessionId).toBeNull();
    expect(state.selectedGameRoomId).toBeNull();
    // 0 件はエラーではない。
    expect(state.error).toBeNull();
    await waitFor(() => {
      expect(window.location.hash).toBe(BASE_TIMELINE_HASH);
    });
    // replace なので履歴は積まれない(thread param は履歴に残らない)。
    expect(window.history.length).toBe(historyLengthBefore);
    view.unmount();
  });

  test('openDirectMessagePane fetches conversation, timeline and status and opens the messages pane', async () => {
    const baseApi = createDesktopMockApi({
      authorSocialViews: {
        [DM_PEER_PUBKEY]: { name: 'dana', following: true, followed_by: true, mutual: true },
      },
    });
    const api = {
      ...baseApi,
      openDirectMessage: vi.fn(baseApi.openDirectMessage),
      listDirectMessageMessages: vi.fn(baseApi.listDirectMessageMessages),
      getDirectMessageStatus: vi.fn(baseApi.getDirectMessageStatus),
    };
    // 既存の会話 1 件を持たせ「先頭挿入」を観測する。
    const { harness, view } = renderRoutingHook({
      hash: BASE_TIMELINE_HASH,
      api,
      preset: (store) => {
        store.getState().patchState({
          directMessages: [buildConversation(OTHER_PEER_PUBKEY)],
        });
      },
    });

    await act(async () => {
      await view.result.current.openDirectMessagePane(DM_PEER_PUBKEY);
    });

    expect(api.openDirectMessage).toHaveBeenCalledTimes(1);
    expect(api.openDirectMessage).toHaveBeenCalledWith(DM_PEER_PUBKEY);
    expect(api.listDirectMessageMessages).toHaveBeenCalledWith(DM_PEER_PUBKEY, null, 100);
    expect(api.getDirectMessageStatus).toHaveBeenCalledWith(DM_PEER_PUBKEY);

    await waitFor(() => {
      expect(harness.store.getState().selectedDirectMessagePeerPubkey).toBe(DM_PEER_PUBKEY);
    });
    const state = harness.store.getState();
    // 取得した会話が先頭に挿入され、既存の会話は残る。
    expect(state.directMessages.map((entry) => entry.peer_pubkey)).toEqual([
      DM_PEER_PUBKEY,
      OTHER_PEER_PUBKEY,
    ]);
    expect(state.directMessages[0]?.peer_name).toBe('dana');
    expect(state.directMessageTimelineByPeer[DM_PEER_PUBKEY]).toEqual([]);
    expect(state.directMessageStatusByPeer[DM_PEER_PUBKEY]).toMatchObject({
      peer_pubkey: DM_PEER_PUBKEY,
      mutual: true,
      send_enabled: true,
    });
    // 会話から知り得た author 情報が knownAuthors に merge される。
    expect(state.knownAuthorsByPubkey[DM_PEER_PUBKEY]).toMatchObject({
      author_pubkey: DM_PEER_PUBKEY,
      name: 'dana',
      mutual: true,
    });
    expect(state.shellChromeState.activePrimarySection).toBe('messages');
    expect(state.shellChromeState.navOpen).toBe(false);
    expect(state.directMessagePaneOpen).toBe(true);
    expect(state.directMessageError).toBeNull();
    expect(state.selectedThread).toBeNull();
    expect(state.selectedAuthorPubkey).toBeNull();
    expect(state.selectedLiveSessionId).toBeNull();
    expect(state.selectedGameRoomId).toBeNull();
    await waitFor(() => {
      expect(window.location.hash).toBe(
        `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${DM_PEER_PUBKEY}`
      );
    });
    view.unmount();
  });

  test('openDirectMessagePane failure with normalizeOnError replaces to a peer-less messages URL', async () => {
    // mutual でない peer への openDirectMessage は mock api が reject する。
    const baseApi = createDesktopMockApi();
    const api = { ...baseApi, openDirectMessage: vi.fn(baseApi.openDirectMessage) };
    const { harness, view } = renderRoutingHook({ hash: BASE_TIMELINE_HASH, api });

    // directMessageError は「設定 → normalize 後の route 同期で null クリア」と遷移するため、
    // 最終値ではなく購読で遷移列そのものを固定する。
    const observedErrors: Array<string | null> = [];
    const unsubscribe = harness.store.subscribe((state, previous) => {
      if (state.directMessageError !== previous.directMessageError) {
        observedErrors.push(state.directMessageError);
      }
    });

    await act(async () => {
      await view.result.current.openDirectMessagePane(STRANGER_PUBKEY, {
        normalizeOnError: true,
      });
    });

    await waitFor(() => {
      expect(observedErrors).toEqual(['direct message requires a mutual relationship', null]);
    });
    await waitFor(() => {
      // peerPubkey param が消えた messages URL へ normalize(replace)される。
      expect(window.location.hash).toBe('#/messages?topic=kukuri%3Atopic%3Ademo');
    });
    const state = harness.store.getState();
    expect(state.directMessagePaneOpen).toBe(true);
    expect(state.selectedDirectMessagePeerPubkey).toBeNull();
    expect(state.shellChromeState.activePrimarySection).toBe('messages');
    unsubscribe();
    view.unmount();
  });

  test('focusPrimarySection updates chrome, clears thread/author/DM panes, and pushes the section URL', async () => {
    const { harness, view } = renderRoutingHook({
      hash:
        '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1' +
        `&authorPubkey=${AUTHOR_PUBKEY}`,
      preset: (store) => {
        store.getState().patchState({
          selectedThread: 'post-1',
          thread: [buildPost({ object_id: 'post-1', root_id: 'post-1' })],
          selectedAuthorPubkey: AUTHOR_PUBKEY,
          selectedAuthor: buildAuthor(AUTHOR_PUBKEY),
        });
        // profileMode / profileConnectionsView を非デフォルト値にして遷移後の挙動を観測する。
        store.getState().patchState({
          shellChromeState: {
            ...store.getState().shellChromeState,
            profileMode: 'edit',
            profileConnectionsView: 'muted',
          },
        });
      },
    });

    // mount 時の route 同期は、profile 以外の route では profileMode を常に 'overview' へ
    // 強制する(useRouteSynchronization.ts L286-318)ため preset の 'edit' は残らない。
    // 一方 profileConnectionsView は現値が引き継がれ preset の 'muted' のまま。
    expect(harness.store.getState().shellChromeState.profileMode).toBe('overview');
    expect(harness.store.getState().shellChromeState.profileConnectionsView).toBe('muted');

    const historyLengthBefore = window.history.length;
    act(() => {
      view.result.current.focusPrimarySection('live');
    });

    await waitFor(() => {
      expect(window.location.hash).toBe('#/live?topic=kukuri%3Atopic%3Ademo');
    });
    const state = harness.store.getState();
    expect(state.shellChromeState.activePrimarySection).toBe('live');
    expect(state.shellChromeState.navOpen).toBe(false);
    // focusPrimarySection は section!=='profile' では profileMode / profileConnectionsView を
    // 触らない(useDesktopShellRouting.ts L509-510)。profileMode は上記 mount 時同期で既に
    // 'overview'、preset した profileConnectionsView='muted' は遷移後もそのまま残る。
    expect(state.shellChromeState.profileMode).toBe('overview');
    expect(state.shellChromeState.profileConnectionsView).toBe('muted');
    expect(state.selectedThread).toBeNull();
    expect(state.thread).toEqual([]);
    expect(state.focusedObjectId).toBeNull();
    expect(state.selectedAuthorPubkey).toBeNull();
    expect(state.selectedAuthor).toBeNull();
    expect(state.authorError).toBeNull();
    expect(state.directMessagePaneOpen).toBe(false);
    expect(state.selectedDirectMessagePeerPubkey).toBeNull();
    // push なので履歴が 1 つ積まれる。
    expect(window.history.length).toBe(historyLengthBefore + 1);
    view.unmount();
  });

  test('toggleNotificationsSection saves the current URL, then returns to lastNonNotificationsRoute', async () => {
    const { harness, view } = renderRoutingHook({ hash: BASE_TIMELINE_HASH });

    // mount 時の route 同期が現 URL を lastNonNotificationsRoute に保存している。
    await waitFor(() => {
      expect(harness.store.getState().lastNonNotificationsRoute).toBe(
        '/timeline?topic=kukuri%3Atopic%3Ademo'
      );
    });

    // 1 回目: notifications 以外にいる → 現 URL を保存して notifications へ。
    act(() => {
      view.result.current.toggleNotificationsSection();
    });
    await waitFor(() => {
      expect(window.location.hash).toBe('#/notifications?topic=kukuri%3Atopic%3Ademo');
    });
    expect(harness.store.getState().shellChromeState.activePrimarySection).toBe('notifications');
    // notifications 滞在中も直前の非 notifications ルートを保持し続ける。
    expect(harness.store.getState().lastNonNotificationsRoute).toBe(
      '/timeline?topic=kukuri%3Atopic%3Ademo'
    );

    // 2 回目: notifications にいる → 保存済みルートへ戻る。
    act(() => {
      view.result.current.toggleNotificationsSection();
    });
    await waitFor(() => {
      expect(window.location.hash).toBe(BASE_TIMELINE_HASH);
    });
    expect(harness.store.getState().shellChromeState.activePrimarySection).toBe('timeline');
    view.unmount();
  });

  test('Escape cascade closes exactly one layer per press: settings -> author -> thread -> nav', async () => {
    // settings drawer + author ペイン + thread ペイン + nav を全て開いた状態から始める。
    // settings=connectivity は store 生成時(createInitialShellState)にも反映されるので
    // shellChromeState のプリセットは navOpen のみでよい。
    const { harness, view } = renderRoutingHook({
      hash:
        '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1' +
        `&authorPubkey=${AUTHOR_PUBKEY}&settings=connectivity`,
      preset: (store) => {
        store.getState().patchState({
          selectedThread: 'post-1',
          thread: [buildPost({ object_id: 'post-1', root_id: 'post-1' })],
          selectedAuthorPubkey: AUTHOR_PUBKEY,
          selectedAuthor: buildAuthor(AUTHOR_PUBKEY),
        });
        store.getState().patchState({
          shellChromeState: { ...store.getState().shellChromeState, navOpen: true },
        });
      },
    });
    expect(harness.store.getState().shellChromeState.settingsOpen).toBe(true);

    const dispatchEscape = () => {
      const event = new KeyboardEvent('keydown', { key: 'Escape', cancelable: true });
      act(() => {
        window.dispatchEvent(event);
      });
      return event;
    };

    // 1 回目: settings drawer だけが閉じる(URL から settings param が消える)。
    const first = dispatchEscape();
    expect(first.defaultPrevented).toBe(true);
    await waitFor(() => {
      expect(harness.store.getState().shellChromeState.settingsOpen).toBe(false);
    });
    await waitFor(() => {
      expect(window.location.hash).toBe(
        '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1' +
          `&authorPubkey=${AUTHOR_PUBKEY}`
      );
    });
    expect(harness.store.getState().selectedAuthorPubkey).toBe(AUTHOR_PUBKEY);
    expect(harness.store.getState().selectedThread).toBe('post-1');
    expect(harness.store.getState().shellChromeState.navOpen).toBe(true);

    // 2 回目: author ペインだけが閉じる。
    const second = dispatchEscape();
    expect(second.defaultPrevented).toBe(true);
    await waitFor(() => {
      expect(harness.store.getState().selectedAuthorPubkey).toBeNull();
    });
    await waitFor(() => {
      expect(window.location.hash).toBe(
        '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1'
      );
    });
    expect(harness.store.getState().selectedAuthor).toBeNull();
    expect(harness.store.getState().selectedThread).toBe('post-1');
    expect(harness.store.getState().shellChromeState.navOpen).toBe(true);

    // 3 回目: thread ペインだけが閉じる。
    const third = dispatchEscape();
    expect(third.defaultPrevented).toBe(true);
    await waitFor(() => {
      expect(harness.store.getState().selectedThread).toBeNull();
    });
    await waitFor(() => {
      expect(window.location.hash).toBe(BASE_TIMELINE_HASH);
    });
    expect(harness.store.getState().thread).toEqual([]);
    expect(harness.store.getState().shellChromeState.navOpen).toBe(true);

    // 4 回目: nav が閉じる(URL は変わらない)。
    const fourth = dispatchEscape();
    expect(fourth.defaultPrevented).toBe(true);
    await waitFor(() => {
      expect(harness.store.getState().shellChromeState.navOpen).toBe(false);
    });
    expect(window.location.hash).toBe(BASE_TIMELINE_HASH);

    // 5 回目: 閉じる対象が無く、イベントは消費されない。
    const fifth = dispatchEscape();
    expect(fifth.defaultPrevented).toBe(false);
    view.unmount();
  });
});
