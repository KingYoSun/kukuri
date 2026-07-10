import type { ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useShellDialogs } from '@/shell/page/useShellDialogs';
import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';

type DialogRouteProps = {
  activePrimarySection: 'timeline' | 'live' | 'game';
  timelineView: 'feed' | 'bookmarks';
};

describe('useShellDialogs', () => {
  test('applies section close rules and loads mention authors when compose opens', async () => {
    const store = createDesktopShellStore();
    const api = createDesktopMockApi();
    const listSocialConnections = vi.spyOn(api, 'listSocialConnections');
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>
    );
    const hook = renderHook(
      ({ activePrimarySection, timelineView }: DialogRouteProps) =>
        useShellDialogs({ activePrimarySection, api, timelineView }),
      {
        initialProps: {
          activePrimarySection: 'timeline',
          timelineView: 'feed',
        },
        wrapper,
      }
    );

    act(() => hook.result.current.setComposeDialogOpen(true));
    await waitFor(() =>
      expect(listSocialConnections).toHaveBeenCalledWith('following')
    );
    hook.rerender({ activePrimarySection: 'timeline', timelineView: 'bookmarks' });
    expect(hook.result.current.composeDialogOpen).toBe(false);

    hook.rerender({ activePrimarySection: 'live', timelineView: 'feed' });
    act(() => hook.result.current.setLiveCreateDialogOpen(true));
    hook.rerender({ activePrimarySection: 'game', timelineView: 'feed' });
    expect(hook.result.current.liveCreateDialogOpen).toBe(false);

    act(() => hook.result.current.setGameCreateDialogOpen(true));
    hook.rerender({ activePrimarySection: 'timeline', timelineView: 'feed' });
    expect(hook.result.current.gameCreateDialogOpen).toBe(false);
  });

  test('owns pending leave state across confirm and cancel', async () => {
    const store = createDesktopShellStore();
    const api = createDesktopMockApi();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>
    );
    const hook = renderHook(
      () =>
        useShellDialogs({
          activePrimarySection: 'timeline',
          api,
          timelineView: 'feed',
        }),
      { wrapper }
    );
    const leaveChannel = vi.fn().mockResolvedValue(undefined);

    act(() => hook.result.current.openLeaveChannelDialog('topic-a', 'channel-a'));
    expect(hook.result.current.leaveChannelDialogOpen).toBe(true);
    await act(async () => hook.result.current.confirmLeaveChannel(leaveChannel));
    expect(leaveChannel).toHaveBeenCalledWith('topic-a', 'channel-a');
    expect(hook.result.current.leaveChannelDialogOpen).toBe(false);

    act(() => hook.result.current.openLeaveChannelDialog('topic-b', 'channel-b'));
    act(() => hook.result.current.setLeaveChannelDialogOpen(false));
    await act(async () => hook.result.current.confirmLeaveChannel(leaveChannel));
    expect(leaveChannel).toHaveBeenCalledTimes(1);
  });
});
