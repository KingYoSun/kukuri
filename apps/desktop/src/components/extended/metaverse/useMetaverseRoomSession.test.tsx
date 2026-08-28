import { act, renderHook, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';

import type {
  DesktopApi,
  DomeHostingView,
  DomePhysicsSnapshotV1,
  GameRoomView,
  SyncStatus,
} from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { METAVERSE_ROOM_RECOVERY_MS, type MetaverseRoomEvent } from '../MetaverseSceneModel';
import { useMetaverseRoomSession } from './useMetaverseRoomSession';
import { createMetaverseRoomActions } from '@/shell/actions/metaverse';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';

const room: GameRoomView = {
  room_id: 'metaverse-room-1',
  host_pubkey: 'f'.repeat(64),
  title: 'Atrium',
  description: 'Small social space',
  status: 'Waiting',
  phase_label: 'fixed-dome-v1',
  scores: [],
  room_kind: 'metaverse_room',
  metaverse: createDefaultMetaverseRoomState(8),
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

function physicsSnapshot(sequence: number, x: number): DomePhysicsSnapshotV1 {
  return {
    instance_id: room.metaverse!.instance_id,
    instance_generation: room.metaverse!.instance_generation,
    lease_epoch: 1,
    session_id: 'session-1',
    host_pubkey: 'e'.repeat(64),
    sequence,
    simulated_at: sequence,
    sleeping: false,
    bodies: [
      {
        entity_id: 'remote-peer',
        kind: 'avatar',
        position: [x, 0, 0],
        rotation: [0, 0, 0],
        linear_velocity: [0, 0, 0],
        animation: 'idle',
        grabbed_by: null,
        expires_at: null,
      },
    ],
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
  initialSelectedRoomId,
}: {
  api?: DesktopApi;
  rooms?: GameRoomView[];
  sync?: SyncStatus;
  onRefresh?: () => Promise<void>;
  initialSelectedRoomId?: string | null;
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
        initialSelectedRoomId,
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
  test('opens the room addressed by the column entity on first render', () => {
    const session = renderSession({ initialSelectedRoomId: room.room_id });

    expect(session.result.current.selectedRoom?.room_id).toBe(room.room_id);
  });

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

  test('closes the room BroadcastChannel on cleanup', async () => {
    const session = renderSession();
    act(() => session.result.current.joinRoom(room.room_id));
    await waitFor(() => expect(MockBroadcastChannel.instances).toHaveLength(1));
    const channel = MockBroadcastChannel.instances[0];
    session.unmount();
    expect(channel.close).toHaveBeenCalledTimes(1);
  });

  test('does not let an out-of-order authoritative snapshot roll back the scene', async () => {
    const api: DesktopApi = {
      ...createDesktopMockApi(),
      submitDomeSessionInput: vi
        .fn()
        .mockResolvedValueOnce(physicsSnapshot(1, 1))
        .mockResolvedValueOnce(physicsSnapshot(3, 3))
        .mockResolvedValueOnce(physicsSnapshot(2, 2)),
    };
    const session = renderSession({ api });
    act(() => session.result.current.joinRoom(room.room_id));
    await waitFor(() =>
      expect(session.result.current.remoteTransforms['remote-peer']?.position[0]).toBe(1)
    );

    act(() => {
      session.result.current.handleLocalTransform({
        roomId: room.room_id,
        peerId: 'local-peer',
        seq: 2,
        position: [0, 0, 0],
        rotation: [0, 0, 0],
        animation: 'idle',
        sentAt: 2,
      });
      session.result.current.handleLocalTransform({
        roomId: room.room_id,
        peerId: 'local-peer',
        seq: 3,
        position: [0, 0, 0],
        rotation: [0, 0, 0],
        animation: 'idle',
        sentAt: 3,
      });
    });

    await waitFor(() =>
      expect(session.result.current.remoteTransforms['remote-peer']?.position[0]).toBe(3)
    );
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

  test('reserves the destination and commits once when the avatar crosses the center line', async () => {
    const domeA = {
      ...room,
      room_id: 'dome-a',
      metaverse: createDefaultMetaverseRoomState(8, {
        roomId: 'dome-a',
        topicId: 'kukuri:topic:demo',
      }),
    };
    const domeB = {
      ...room,
      room_id: 'dome-b',
      title: 'Neighbor',
      metaverse: createDefaultMetaverseRoomState(8, {
        roomId: 'dome-b',
        topicId: 'kukuri:topic:demo',
      }),
    };
    const hosted = (instanceId: string): DomeHostingView => ({
      instance_id: instanceId,
      state: {
        kind: 'community_node_hosted',
        host: { kind: 'community_node', node_id: 'cn-1', api_base_url: 'https://cn.example' },
        lease_id: `lease-${instanceId}`,
        lease_epoch: 1,
        lease_expires_at: Date.now() + 60_000,
        session_id: `session-${instanceId}`,
        reason: null,
        last_heartbeat_at: Date.now(),
      },
      lease: null,
      signed_lease_json: null,
      signed_activation_json: null,
      signed_close_json: null,
      instance_manifest_json: '{}',
      preset_manifest_json: '{}',
      participants: 0,
      sleeping: false,
      resource_budget: {} as DomeHostingView['resource_budget'],
      resource_metrics: {} as DomeHostingView['resource_metrics'],
    });
    const prepareDomeTransition = vi.fn(async (request) => ({
      request,
      target_lease_epoch: 1,
      target_session_id: 'session-dome-b',
      expires_at: Date.now() + 15_000,
    }));
    const commitDomeTransition = vi.fn().mockResolvedValue(undefined);
    const abortDomeTransition = vi.fn().mockResolvedValue(undefined);
    const api: DesktopApi = {
      ...createDesktopMockApi(),
      listDomeConnectionTopology: vi.fn().mockResolvedValue({
        proposals: [],
        connections: [{
          record: {
            agreement: {
              connection_id: 'connection-1',
              proposal_id: 'proposal-1',
              spatial_context: domeA.metaverse.spatial_context,
              proposer: {
                instance_id: 'dome-a',
                instance_generation: 1,
                owner_pubkey: domeA.host_pubkey,
                direction: 'north',
              },
              receiver: {
                instance_id: 'dome-b',
                instance_generation: 1,
                owner_pubkey: domeB.host_pubkey,
                direction: 'south',
              },
              activation_generation: 1,
            },
            receiver_slot_generation: 1,
            observed_active_connection_ids: [],
            status: 'active',
            lifecycle_generation: 1,
            lifecycle_actor: null,
            lifecycle_reason: null,
          },
        }],
        resolution: {
          topology: {
            spatial_context: domeA.metaverse.spatial_context,
            components: [{
              root_instance_id: 'dome-a',
              instance_ids: ['dome-a', 'dome-b'],
              connection_ids: ['connection-1'],
              coordinates_cm: { 'dome-a': [0, 0, 0], 'dome-b': [0, 0, -5_700] },
            }],
            active_connection_ids: ['connection-1'],
            topology_digest: 'topology-1',
          },
          rejected_connections: [],
        },
      }),
      getDomeHosting: vi.fn(async (_context, instanceId) => hosted(instanceId)),
      submitDomeSessionInput: vi.fn(async (_context, instanceId, sequence) => ({
        instance_id: instanceId,
        instance_generation: 1,
        lease_epoch: 1,
        session_id: `session-${instanceId}`,
        host_pubkey: 'e'.repeat(64),
        sequence,
        simulated_at: Date.now(),
        sleeping: false,
        bodies: [],
      })),
      prepareDomeTransition,
      commitDomeTransition,
      abortDomeTransition,
    };
    const session = renderSession({
      api,
      rooms: [domeA, domeB],
      initialSelectedRoomId: 'dome-a',
    });
    await waitFor(() =>
      expect(session.result.current.transitionNeighbors[0]?.boundaryState).toBe('ready')
    );

    act(() => session.result.current.handleLocalTransform({
      roomId: 'dome-a',
      peerId: 'local-peer',
      seq: 1,
      position: [0, 90, -2_200],
      rotation: [0, 0, 0],
      animation: 'walk',
      sentAt: Date.now(),
    }));
    await waitFor(() => expect(prepareDomeTransition).toHaveBeenCalledTimes(1));
    act(() => session.result.current.handleLocalTransform({
      roomId: 'dome-a',
      peerId: 'local-peer',
      seq: 2,
      position: [0, 90, -2_870],
      rotation: [0, 0, 0],
      animation: 'walk',
      sentAt: Date.now(),
    }));

    await waitFor(() => expect(session.result.current.selectedRoomId).toBe('dome-b'));
    expect(commitDomeTransition).toHaveBeenCalledTimes(1);
    expect(commitDomeTransition).toHaveBeenCalledWith(
      expect.objectContaining({ target_session_id: 'session-dome-b' }),
      [0, 90, 2_830],
      [0, 0, 0]
    );
    expect(abortDomeTransition).not.toHaveBeenCalled();
  });
});
