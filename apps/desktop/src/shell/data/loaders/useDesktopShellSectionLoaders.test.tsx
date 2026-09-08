import type { ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { describe, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { useDesktopShellSectionLoaders } from '@/shell/data/loaders/useDesktopShellSectionLoaders';
import { useNotificationLoaders } from '@/shell/data/loaders/useNotificationLoaders';
import { privateTimelineScope } from '@/shell/presentation';
import {
  createDesktopShellStore,
  DesktopShellStoreContext,
} from '@/shell/store';
import { columnIdentityId, openTransientColumn } from '@/shell/slices/workspace';
import { createDeferred } from '@/shell/DesktopShellPage.testHelpers';
import type { CommunityNodeManifestFetch, TimelineView } from '@/lib/api';

function setup() {
  const api = createDesktopMockApi();
  const store = createDesktopShellStore();
  const loadReactionCatalogData = vi.fn().mockResolvedValue(undefined);
  const translate = (key: string) => key;
  const wrapper = ({ children }: { children: ReactNode }) => (
    <DesktopShellStoreContext.Provider value={store}>{children}</DesktopShellStoreContext.Provider>
  );
  const hook = renderHook(
    () => {
      const { loadNotificationsSection } = useNotificationLoaders({
        api, translate, activePrimarySection: 'notifications',
      });
      return useDesktopShellSectionLoaders({
        api,
        loadReactionCatalogData,
        loadNotificationsSection,
        storeApi: store,
        translate,
      });
    },
    { wrapper }
  );
  return { api, hook, store };
}

describe('useDesktopShellSectionLoaders', () => {
  test('manifest completion resolves the index using consent received after the request started', async () => {
    const { api, hook, store } = setup();
    const node = 'https://first.example';
    const config = await api.setCommunityNodeConfig([{ base_url: node }]);
    const pending = await api.getCommunityNodeStatuses();
    const manifest = await api.fetchCommunityNodeManifest(node);
    store.getState().patchState({ communityNodeConfig: config, communityNodeStatuses: pending });
    const deferred = createDeferred<CommunityNodeManifestFetch>();
    const fetch = vi.spyOn(api, 'fetchCommunityNodeManifest').mockReturnValue(deferred.promise);
    let loading!: Promise<void>;
    act(() => { loading = hook.result.current.loadCommunityIndexCapability(); });
    await waitFor(() => expect(fetch).toHaveBeenCalled());
    const policies = await api.fetchCommunityNodePolicies(node);
    const ready = await api.acceptCommunityNodeConsents(node, policies.policies, 'en');
    act(() => store.getState().setField('communityNodeStatuses', [ready]));
    await act(async () => { deferred.resolve(manifest); await loading; });
    expect(store.getState().communityIndexNodeBaseUrl).toBe(node);
  });

  test('an older manifest response cannot overwrite a newer capability result', async () => {
    const { api, hook, store } = setup();
    const node = 'https://first.example';
    await api.setCommunityNodeConfig([{ base_url: node }]);
    const policies = await api.fetchCommunityNodePolicies(node);
    const ready = await api.acceptCommunityNodeConsents(node, policies.policies, 'en');
    store.getState().setField('communityNodeStatuses', [ready]);
    const manifest = await api.fetchCommunityNodeManifest(node);
    const deferred = createDeferred<CommunityNodeManifestFetch>();
    const fetch = vi.spyOn(api, 'fetchCommunityNodeManifest')
      .mockReturnValueOnce(deferred.promise).mockResolvedValue(manifest);
    let first!: Promise<void>;
    act(() => { first = hook.result.current.loadCommunityIndexCapability(); });
    await waitFor(() => expect(fetch).toHaveBeenCalledTimes(1));
    await act(async () => hook.result.current.loadCommunityIndexCapability());
    await act(async () => { deferred.resolve({ status: 'absent', manifest: null }); await first; });
    expect(store.getState().communityNodeManifests[node]?.status).toBe('ok');
    expect(store.getState().communityIndexNodeBaseUrl).toBe(node);
  });

  test('loads an inactive own profile and preserves an unsaved profile draft', async () => {
    const { api, hook, store } = setup();
    const profile = await api.getMyProfile();
    const objectId = await api.createPost('kukuri:topic:general', 'saved public post');
    const activeColumnId = store.getState().workspaceState.activeColumnId;
    store.setState({ profileDirty: true, profileDraft: { display_name: 'unsaved name' } });

    await act(async () => hook.result.current.loadShellSections('kukuri:topic:general'));

    expect(store.getState().profileTimeline.map((post) => post.object_id)).toContain(objectId);
    expect(store.getState().localProfile?.pubkey).toBe(profile.pubkey);
    expect(store.getState().profileDraft.display_name).toBe('unsaved name');
    expect(store.getState().workspaceState.activeColumnId).toBe(activeColumnId);
  });

  test.each(['success', 'failure'])('ignores an older profile %s after a newer response', async (outcome) => {
    const { api, hook, store } = setup();
    const profile = await api.getMyProfile();
    await api.createPost('kukuri:topic:general', 'newest profile post');
    const latest = await api.listProfileTimeline(profile.pubkey);
    const old = createDeferred<TimelineView>();
    const listProfileTimeline = vi.spyOn(api, 'listProfileTimeline')
      .mockReturnValueOnce(old.promise)
      .mockResolvedValueOnce(latest);
    let first!: Promise<void>;
    act(() => { first = hook.result.current.loadProfileSection(); });
    await waitFor(() => expect(listProfileTimeline).toHaveBeenCalledTimes(1));
    await act(async () => hook.result.current.loadProfileSection());
    await act(async () => {
      if (outcome === 'success') old.resolve({ items: [], next_cursor: null });
      else old.reject(new Error('outdated failure'));
      await first;
    });
    expect(store.getState().profileTimeline).toEqual(latest.items);
    expect(store.getState().profilePanelState).toEqual({ status: 'ready', error: null });
  });

  test('retains confirmed posts on profile failure and recovers on retry', async () => {
    const { api, hook, store } = setup();
    await api.createPost('kukuri:topic:general', 'retained public post');
    await act(async () => hook.result.current.loadProfileSection());
    const confirmed = store.getState().profileTimeline;
    vi.spyOn(api, 'listProfileTimeline').mockRejectedValueOnce(new Error('profile read failed'));
    await act(async () => hook.result.current.loadProfileSection());
    expect(store.getState().profileTimeline).toEqual(confirmed);
    expect(store.getState().profileError).toBe('profile read failed');
    await act(async () => hook.result.current.loadProfileSection());
    expect(store.getState().profileTimeline).toEqual(confirmed);
    expect(store.getState().profileError).toBeNull();
  });

  test('refreshing an own-author column does not replace another selected author', async () => {
    const { api, hook, store } = setup();
    const profile = await api.getMyProfile();
    await api.createPost('kukuri:topic:general', 'own-author refresh');
    const ownTimeline = await api.listProfileTimeline(profile.pubkey);
    const otherPubkey = 'b'.repeat(64);
    const other = { ...ownTimeline.items[0], object_id: 'other-post', author_pubkey: otherPubkey };
    store.setState({ selectedAuthorPubkey: otherPubkey, selectedAuthorTimeline: [other] });
    await act(async () => hook.result.current.loadAuthorSection(profile.pubkey));
    expect(store.getState().selectedAuthorTimeline).toEqual([other]);
    expect(store.getState().authorTimelinesByPubkey[profile.pubkey]).toEqual(ownTimeline.items);
  });

  test('does not fetch an own profile when its column is closed', async () => {
    const { api, hook, store } = setup();
    store.setState((state) => ({ workspaceState: {
      ...state.workspaceState,
      columns: state.workspaceState.columns.filter((column) => column.kind !== 'profile'),
    } }));
    const read = vi.spyOn(api, 'listProfileTimeline');
    await act(async () => hook.result.current.loadShellSections('kukuri:topic:general'));
    expect(read).not.toHaveBeenCalled();
  });

  test('loads only the active live section with the selected channel scope', async () => {
    const { api, hook, store } = setup();
    const listLiveSessions = vi.spyOn(api, 'listLiveSessions').mockResolvedValue([]);
    const listGameRooms = vi.spyOn(api, 'listGameRooms');
    store.setState((state) => ({
      workspaceState: openTransientColumn(state.workspaceState, {
        id: columnIdentityId('stream', { topicId: 'topic', channelId: 'channel-a' }),
        kind: 'stream',
        scope: { topicId: 'topic', channelId: 'channel-a' },
        pinned: false,
      }),
    }));

    await act(async () => hook.result.current.loadShellSections('topic'));

    expect(listLiveSessions).toHaveBeenCalledWith(
      'topic',
      privateTimelineScope('channel-a')
    );
    expect(store.getState().livePanelStateByScopeKey['topic::channel::channel-a']).toEqual({
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
      workspaceState: openTransientColumn(state.workspaceState, {
        id: columnIdentityId('conversation', { topicId: 'topic', channelId: null }, peer),
        kind: 'conversation',
        scope: { topicId: 'topic', channelId: null },
        entityId: peer,
        pinned: false,
      }),
    }));
    vi.spyOn(api, 'listDirectMessages').mockResolvedValue([]);
    vi.spyOn(api, 'listDirectMessageMessages').mockRejectedValue(new Error('timeline failed'));
    vi.spyOn(api, 'getDirectMessageStatus').mockResolvedValue(status);

    await act(async () => hook.result.current.loadShellSections('topic'));

    expect(store.getState().directMessageStatusByPeer[peer]).toEqual(status);
    expect(store.getState().directMessageError).toBe('timeline failed');
  });

  test('uses the localized DM load fallback for a non-Error failure', async () => {
    const { api, hook, store } = setup();
    store.setState((state) => ({
      workspaceState: openTransientColumn(state.workspaceState, {
        id: columnIdentityId('messages', { topicId: 'topic', channelId: null }),
        kind: 'messages',
        scope: { topicId: 'topic', channelId: null },
        pinned: false,
      }),
    }));
    vi.spyOn(api, 'listDirectMessages').mockRejectedValue(null);

    await act(async () => hook.result.current.loadShellSections('topic'));

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

    await act(async () => hook.result.current.loadShellSections('topic'));

    expect(store.getState().discoveryConfig).toEqual(config);
    expect(store.getState().discoverySeedInput).toBe('keep-local-editor');
  });
});
