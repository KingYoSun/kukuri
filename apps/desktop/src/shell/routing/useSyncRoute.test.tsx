/**
 * WP-S5: routing/useSyncRoute の characterization テスト。
 *
 * 後続 WP-H6(shell store のスライス化・selector 化・prop drilling 除去)の
 * 安全網として「現時点で観測した挙動」をそのまま固定する。
 * このテストが落ちた場合、疑うべきは加えた変更でありテストではない。
 *
 * 方針:
 * - 依存 4 つ(navigate / pendingRouteUrlRef / resolvedRouteLocation / storeApi)は
 *   全て引数注入のため wrapper 不要。navigate は vi.fn を渡す。
 * - overrides の Object.prototype.hasOwnProperty 判定(キー省略 = store の現状維持 /
 *   null 指定 = クリア / キーを明示して undefined を渡した場合も `?? null` によりクリア扱い。
 *   観測挙動として固定する)が適用されるのは、nullable フィールド群(selectedThread /
 *   focusedObjectId / selectedAuthorPubkey / selectedDirectMessagePeerPubkey /
 *   selectedLiveSessionId / selectedGameRoomId)と composeTarget / timelineScope /
 *   settingsOpen(こちらは `?? false`)のみ(useSyncRoute.ts L38-58, L63-73)。
 *   activeTopic / primarySection / timelineView / profileMode / profileConnectionsView /
 *   settingsSection は素の `??` 判定で、明示 undefined はキー省略と同じ現状維持になる
 *   (useSyncRoute.ts L29-37, L59-60)。
 * - 生成 URL が現 URL と同一なら navigate せず pendingRouteUrlRef を null に戻し、
 *   異なれば pendingRouteUrlRef に次 URL を記録してから navigate する。
 * - 固定するのは navigate の呼び出し引数と pendingRouteUrlRef の値のみ。
 */
import { renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { useSyncRoute } from '@/shell/routing/useSyncRoute';
import { createDesktopShellStore, type DesktopShellStoreApi } from '@/shell/store';
import { resetWindowHash } from '@/shell/testSupport/renderShellHook';
import { columnIdentityId, openTransientColumn } from '@/shell/slices/workspace';

type SyncRouteArgs = Parameters<typeof useSyncRoute>[0];

/**
 * 引数一式を生成する。resolvedRouteLocation の既定は search なしの '/timeline' で、
 * buildShellUrl の出力(常に ?topic= を含む)とは一致しないため navigate が発火する。
 */
function createHookArgs(
  storeApi: DesktopShellStoreApi,
  overrides: Partial<SyncRouteArgs> = {}
): SyncRouteArgs {
  return {
    navigate: vi.fn(),
    pendingRouteUrlRef: { current: null },
    resolvedRouteLocation: { pathname: '/timeline', search: '' },
    storeApi,
    ...overrides,
  };
}

describe('useSyncRoute', () => {
  beforeEach(() => {
    resetWindowHash();
  });

  describe('overrides own-property semantics', () => {
    test('keeps current store selections in the url when override keys are omitted', () => {
      const storeApi = createDesktopShellStore();
      storeApi.getState().patchState({
        selectedThread: 'post-1',
        focusedObjectId: 'obj-1',
        workspaceState: openTransientColumn(storeApi.getState().workspaceState, {
          id: columnIdentityId(
            'thread',
            { topicId: 'kukuri:topic:demo', channelId: null },
            'post-1'
          ),
          kind: 'thread',
          scope: { topicId: 'kukuri:topic:demo', channelId: null },
          entityId: 'post-1',
          pinned: false,
        }),
      });
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('push');

      expect(args.navigate).toHaveBeenCalledTimes(1);
      expect(args.navigate).toHaveBeenCalledWith(
        '/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1&focusObjectId=obj-1',
        { replace: false }
      );
      expect(args.pendingRouteUrlRef.current).toBe(
        '/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-1&focusObjectId=obj-1'
      );
      view.unmount();
    });

    test('clears selections in the url when overrides pass null explicitly', () => {
      const storeApi = createDesktopShellStore();
      storeApi.getState().patchState({ selectedThread: 'post-1', focusedObjectId: 'obj-1' });
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('replace', { focusedObjectId: null, selectedThread: null });

      expect(args.navigate).toHaveBeenCalledTimes(1);
      expect(args.navigate).toHaveBeenCalledWith('/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: true,
      });
      view.unmount();
    });

    test('treats an explicitly-passed undefined override the same as null via hasOwnProperty', () => {
      const storeApi = createDesktopShellStore();
      storeApi.getState().patchState({ selectedThread: 'post-1' });
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      // キーが存在する時点で hasOwnProperty は true になり、undefined ?? null → null(クリア)
      view.result.current('replace', { selectedThread: undefined });

      expect(args.navigate).toHaveBeenCalledTimes(1);
      expect(args.navigate).toHaveBeenCalledWith('/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: true,
      });
      view.unmount();
    });
  });

  describe('url comparison and pendingRouteUrlRef', () => {
    test('does not navigate and resets pendingRouteUrlRef to null when the built url equals the current url', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi, {
        pendingRouteUrlRef: { current: '/timeline?topic=stale' },
        resolvedRouteLocation: { pathname: '/timeline', search: '?topic=kukuri%3Atopic%3Ademo' },
      });
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('replace');

      expect(args.navigate).not.toHaveBeenCalled();
      expect(args.pendingRouteUrlRef.current).toBeNull();
      view.unmount();
    });

    test('records the next url into pendingRouteUrlRef and navigates with replace by default', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      // mode 省略時は 'replace'
      view.result.current();

      expect(args.pendingRouteUrlRef.current).toBe('/timeline?topic=kukuri%3Atopic%3Ademo');
      expect(args.navigate).toHaveBeenCalledTimes(1);
      expect(args.navigate).toHaveBeenCalledWith('/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: true,
      });
      view.unmount();
    });

    test('navigates with replace=false when mode is push', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('push');

      expect(args.navigate).toHaveBeenCalledWith('/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: false,
      });
      view.unmount();
    });
  });

  describe('channel selection overrides', () => {
    test('derives the channel param from a composeTarget override', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('replace', {
        composeTarget: { kind: 'private_channel', channel_id: 'chan-9' },
      });

      expect(args.navigate).toHaveBeenCalledWith(
        '/timeline?topic=kukuri%3Atopic%3Ademo&channel=chan-9',
        { replace: true }
      );
      view.unmount();
    });

    test('derives the channel param from a timelineScope override when composeTarget is absent', () => {
      const storeApi = createDesktopShellStore();
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      view.result.current('replace', {
        timelineScope: { kind: 'channel', channel_id: 'chan-7' },
      });

      expect(args.navigate).toHaveBeenCalledWith(
        '/timeline?topic=kukuri%3Atopic%3Ademo&channel=chan-7',
        { replace: true }
      );
      view.unmount();
    });

    test('a public composeTarget override drops the channel param even when the store has a selected channel', () => {
      const storeApi = createDesktopShellStore();
      storeApi.getState().patchState({
        workspaceState: openTransientColumn(storeApi.getState().workspaceState, {
          id: columnIdentityId('timeline', {
            topicId: 'kukuri:topic:demo',
            channelId: 'chan-1',
          }),
          kind: 'timeline',
          scope: { topicId: 'kukuri:topic:demo', channelId: 'chan-1' },
          pinned: false,
        }),
      });
      const args = createHookArgs(storeApi);
      const view = renderHook(() => useSyncRoute(args));

      // override なしでは store の選択 channel が URL に載る
      view.result.current('push');
      expect(args.navigate).toHaveBeenNthCalledWith(
        1,
        '/timeline?topic=kukuri%3Atopic%3Ademo&channel=chan-1',
        { replace: false }
      );

      // public composeTarget を明示すると channel が落ちる
      view.result.current('replace', { composeTarget: { kind: 'public' } });
      expect(args.navigate).toHaveBeenNthCalledWith(2, '/timeline?topic=kukuri%3Atopic%3Ademo', {
        replace: true,
      });
      view.unmount();
    });
  });
});
