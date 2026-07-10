import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import type { DesktopApi, GameRoomView, SyncStatus } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  DEFAULT_SHARED_OBJECT,
  METAVERSE_ROOM_RECOVERY_MS,
  type MetaverseRoomEvent,
} from '../MetaverseSceneModel';
import { useMetaverseRoomSession } from './useMetaverseRoomSession';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';

const room: GameRoomView = {
  room_id: 'metaverse-room-1',
  host_pubkey: 'f'.repeat(64),
  title: 'Atrium',
  description: 'Small social space',
  status: 'Waiting',
  phase_label: 'metaverse-mvp',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: {
    world_version: 1,
    max_peers: 8,
    scene: {
      ground: 'default',
      shared_object: DEFAULT_SHARED_OBJECT,
    },
    default_spawn: {
      position: [0, 0, 260],
      rotation: [0, 180, 0],
    },
    asset_refs: [],
    chat_history: [],
  },
  manifest_blob_hash: 'manifest-1',
  updated_at: 1,
  channel_id: null,
  audience_label: 'Public',
};

function syncStatus(connected = true): SyncStatus {
  return {
    connected,
    delivery_state: connected ? 'Live' : 'Offline',
    peer_count: connected ? 1 : 0,
    pending_events: 0,
    status_detail: connected ? 'connected' : 'offline',
    configured_peers: [],
    subscribed_topics: ['kukuri:topic:demo'],
    active_path: 'direct_p2p',
    fallback_peer_ids: [],
    topic_diagnostics: [],
    local_author_pubkey: 'f'.repeat(64),
    discovery: {
      mode: 'seeded_dht',
      connect_mode: 'direct_only',
      active_path: 'direct_p2p',
      fallback_peer_ids: [],
      env_locked: false,
      configured_seed_peer_ids: [],
      bootstrap_seed_peer_ids: [],
      manual_ticket_peer_ids: [],
      connected_peer_ids: [],
      docs_assist_peer_ids: [],
      blob_assist_peer_ids: [],
      local_endpoint_id: 'local-endpoint-a',
    },
    gossip_disabled_topics: [],
    gossip_disabled_channels: [],
  };
}

class MockBroadcastChannel {
  static instances: MockBroadcastChannel[] = [];

  onmessage: ((event: MessageEvent<MetaverseRoomEvent>) => void) | null = null;
  readonly postMessage = vi.fn();
  readonly close = vi.fn();

  constructor(readonly name: string) {
    MockBroadcastChannel.instances.push(this);
  }
}

type SessionProps = {
  rooms: GameRoomView[];
  sync: SyncStatus;
};

function renderSession({
  api = createDesktopMockApi(),
  rooms = [room],
  sync = syncStatus(),
  onRefresh = vi.fn().mockResolvedValue(undefined),
}: {
  api?: DesktopApi;
  rooms?: GameRoomView[];
  sync?: SyncStatus;
  onRefresh?: () => Promise<void>;
} = {}) {
  const onError = vi.fn();
  const actions = createMetaverseRoomActions({
    api,
    activeTopic: 'kukuri:topic:demo',
    activeComposeChannel: { kind: 'public' },
    onRefresh,
  });
  const rendered = renderHook(
    ({ rooms: currentRooms, sync: currentSync }: SessionProps) =>
      useMetaverseRoomSession({
        actions,
        activeTopic: 'kukuri:topic:demo',
        rooms: currentRooms,
        syncStatus: currentSync,
        locale: 'en',
        localDisplayName: 'Local Author',
        localAvatarAssetRef: null,
        localAvatarAssetUrl: null,
        onError,
      }),
    { initialProps: { rooms, sync } }
  );
  return { ...rendered, api, onRefresh, onError };
}

beforeEach(() => {
  MockBroadcastChannel.instances = [];
  vi.stubGlobal(
    'BroadcastChannel',
    MockBroadcastChannel as unknown as typeof BroadcastChannel
  );
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe('useMetaverseRoomSession', () => {
  test('clears a missing selected room but preserves a pending created-room selection', async () => {
    const session = renderSession();
    act(() => session.result.current.joinRoom(room.room_id));
    expect(session.result.current.selectedRoom?.room_id).toBe(room.room_id);

    session.rerender({ rooms: [], sync: syncStatus() });
    await waitFor(() => expect(session.result.current.selectedRoomId).toBeNull());

    act(() => session.result.current.selectCreatedRoom('created-room'));
    expect(session.result.current.selectedRoomId).toBe('created-room');
    session.rerender({ rooms: [], sync: syncStatus() });
    expect(session.result.current.selectedRoomId).toBe('created-room');

    const createdRoom = { ...room, room_id: 'created-room', title: 'Created Room' };
    session.rerender({ rooms: [createdRoom], sync: syncStatus() });
    expect(session.result.current.selectedRoom?.room_id).toBe('created-room');
    session.rerender({ rooms: [], sync: syncStatus() });
    await waitFor(() => expect(session.result.current.selectedRoomId).toBeNull());
  });

  test('merges newer BroadcastChannel object updates, rejects stale updates, and closes on cleanup', async () => {
    const session = renderSession();
    act(() => session.result.current.joinRoom(room.room_id));
    await waitFor(() => expect(MockBroadcastChannel.instances).toHaveLength(1));
    const channel = MockBroadcastChannel.instances[0];
    const newerObject = {
      ...DEFAULT_SHARED_OBJECT,
      position: [100, 0, 0] as [number, number, number],
      updated_by: 'remote-peer',
      updated_at: 10,
    };
    const staleObject = {
      ...DEFAULT_SHARED_OBJECT,
      position: [-100, 0, 0] as [number, number, number],
      updated_by: 'remote-peer',
      updated_at: 5,
    };

    act(() => {
      channel.onmessage?.({
        data: { type: 'object.update', roomId: room.room_id, object: newerObject },
      } as MessageEvent<MetaverseRoomEvent>);
      channel.onmessage?.({
        data: { type: 'object.update', roomId: room.room_id, object: staleObject },
      } as MessageEvent<MetaverseRoomEvent>);
    });

    expect(session.result.current.sharedObject).toEqual(newerObject);
    session.unmount();
    expect(channel.close).toHaveBeenCalledTimes(1);
  });

  test('marks the room offline after three consecutive backend poll failures', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(100_000);
    const api: DesktopApi = {
      ...createDesktopMockApi(),
      listMetaverseRoomEvents: vi.fn().mockRejectedValue(new Error('poll failed')),
    };
    const session = renderSession({ api });
    act(() => session.result.current.joinRoom(room.room_id));

    await act(async () => {
      await Promise.resolve();
      await vi.advanceTimersByTimeAsync(400);
    });

    expect(api.listMetaverseRoomEvents).toHaveBeenCalledTimes(3);
    expect(session.result.current.roomConnectionState).toBe('offline');
  });

  test('enforces recovery cooldown and clears heartbeat/poll timers on cleanup', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(100_000);
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const clearInterval = vi.spyOn(window, 'clearInterval');
    const clearTimeout = vi.spyOn(window, 'clearTimeout');
    const session = renderSession({ onRefresh });
    act(() => session.result.current.joinRoom(room.room_id));
    await act(async () => Promise.resolve());

    session.rerender({ rooms: [room], sync: syncStatus(false) });
    await act(async () => Promise.resolve());
    expect(onRefresh).toHaveBeenCalledTimes(1);

    session.rerender({ rooms: [room], sync: syncStatus(true) });
    session.rerender({ rooms: [room], sync: syncStatus(false) });
    await act(async () => Promise.resolve());
    expect(onRefresh).toHaveBeenCalledTimes(1);

    vi.setSystemTime(100_000 + METAVERSE_ROOM_RECOVERY_MS + 1);
    session.rerender({ rooms: [room], sync: syncStatus(true) });
    session.rerender({ rooms: [room], sync: syncStatus(false) });
    await act(async () => Promise.resolve());
    expect(onRefresh).toHaveBeenCalledTimes(2);

    session.unmount();
    expect(clearInterval).toHaveBeenCalled();
    expect(clearTimeout).toHaveBeenCalled();
  });
});
