import type { ReactNode } from 'react';
import { act, renderHook } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useDesktopShellSectionLoaders } from '@/shell/data/loaders/useDesktopShellSectionLoaders';
import { privateTimelineScope } from '@/shell/presentation';
import {
  createDesktopShellStore,
  DesktopShellStoreContext,
} from '@/shell/store';

function setup() {
  const api = createDesktopMockApi();
  const store = createDesktopShellStore();
  const loadReactionCatalogData = vi.fn().mockResolvedValue(undefined);
  const wrapper = ({ children }: { children: ReactNode }) => (
    <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>
  );
  const hook = renderHook(
    () =>
      useDesktopShellSectionLoaders({
        api,
        loadReactionCatalogData,
        storeApi: store,
        translate: (key) => key,
      }),
    { wrapper }
  );
  return { api, hook, store };
}

describe('useDesktopShellSectionLoaders', () => {
  test('loads only the active live section with the selected channel scope', async () => {
    const { api, hook, store } = setup();
    const listLiveSessions = vi.spyOn(api, 'listLiveSessions').mockResolvedValue([]);
    const listGameRooms = vi.spyOn(api, 'listGameRooms');
    store.setState((state) => ({
      selectedChannelIdByTopic: { topic: 'channel-a' },
      shellChromeState: { ...state.shellChromeState, activePrimarySection: 'live' },
    }));

    await act(async () => hook.result.current('topic'));

    expect(listLiveSessions).toHaveBeenCalledWith(
      'topic',
      privateTimelineScope('channel-a')
    );
    expect(store.getState().livePanelStateByTopic.topic).toEqual({
      status: 'ready',
      error: null,
    });
    expect(listGameRooms).not.toHaveBeenCalled();
  });

  test('keeps fulfilled DM status when the timeline branch fails', async () => {
    const { api, hook, store } = setup();
    const peer = 'b'.repeat(64);
    const status = await api.getDirectMessageStatus(peer);
    store.setState((state) => ({
      selectedDirectMessagePeerPubkey: peer,
      shellChromeState: { ...state.shellChromeState, activePrimarySection: 'messages' },
    }));
    vi.spyOn(api, 'listDirectMessages').mockResolvedValue([]);
    vi.spyOn(api, 'listDirectMessageMessages').mockRejectedValue(new Error('timeline failed'));
    vi.spyOn(api, 'getDirectMessageStatus').mockResolvedValue(status);

    await act(async () => hook.result.current('topic'));

    expect(store.getState().directMessageStatusByPeer[peer]).toEqual(status);
    expect(store.getState().directMessageError).toBe('timeline failed');
  });

  test('uses the localized DM load fallback for a non-Error failure', async () => {
    const { api, hook, store } = setup();
    store.setState((state) => ({
      shellChromeState: { ...state.shellChromeState, activePrimarySection: 'messages' },
    }));
    vi.spyOn(api, 'listDirectMessages').mockRejectedValue(null);

    await act(async () => hook.result.current('topic'));

    expect(store.getState().directMessageError).toBe(
      'common:errors.failedToLoadDirectMessages'
    );
  });

  test('refreshes discovery config without overwriting a dirty editor', async () => {
    const { api, hook, store } = setup();
    const config = await api.getDiscoveryConfig();
    store.setState((state) => ({
      discoveryEditorDirty: true,
      discoverySeedInput: 'keep-local-editor',
      shellChromeState: {
        ...state.shellChromeState,
        settingsOpen: true,
        activeSettingsSection: 'discovery',
      },
    }));
    vi.spyOn(api, 'getDiscoveryConfig').mockResolvedValue(config);

    await act(async () => hook.result.current('topic'));

    expect(store.getState().discoveryConfig).toEqual(config);
    expect(store.getState().discoverySeedInput).toBe('keep-local-editor');
  });
});
