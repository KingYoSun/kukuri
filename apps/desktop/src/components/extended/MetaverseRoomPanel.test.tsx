import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import type {
  DesktopApi,
  GameRoomView,
  MetaverseRoomEventView,
  SharedRoomObjectV1,
  SyncStatus,
} from '@/lib/api';
import { MetaverseRoomPanel } from './MetaverseRoomPanel';

vi.mock('./MetaverseScene', () => ({
  MetaverseScene: (props: {
    room: GameRoomView;
    localPeerId: string;
    sharedObject: SharedRoomObjectV1;
    onLocalTransform: (transform: {
      roomId: string;
      peerId: string;
      seq: number;
      position: [number, number, number];
      rotation: [number, number, number];
      animation: 'idle' | 'walk' | 'sprint' | 'jump' | 'sitting';
      sentAt: number;
    }) => void;
    onAvatarAssetStatus: (status: 'loading' | 'sample-vrm' | 'blob-vrm' | 'fallback-primitive') => void;
  }) => (
    <div aria-label='Metaverse room viewport'>
      <button
        type='button'
        onClick={() =>
          props.onLocalTransform({
            roomId: props.room.room_id,
            peerId: props.localPeerId,
            seq: 12,
            position: [10, 0, 20],
            rotation: [0, 90, 0],
            animation: 'sprint',
            sentAt: 42,
          })
        }
      >
        Emit sprint transform
      </button>
      <button type='button' onClick={() => props.onAvatarAssetStatus('fallback-primitive')}>
        Mark animation fallback
      </button>
      <span>{props.sharedObject.object_id}</span>
    </div>
  ),
}));

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
      shared_object: {
        object_id: 'mvp-object-1',
        asset_ref: null,
        primitive_fallback: 'cube',
        position: [0, 50, -240],
        rotation: [0, 0, 0],
        scale: [100, 100, 100],
        updated_by: 'f'.repeat(64),
        updated_at: 1,
      },
    },
    default_spawn: {
      position: [0, 0, 260],
      rotation: [0, 180, 0],
    },
    asset_refs: [],
  },
  manifest_blob_hash: 'mock-metaverse-room-1',
  updated_at: 1,
  channel_id: null,
  audience_label: 'Public',
};

function avatarEvent(peerId: string, animation: string | null, seq: number): MetaverseRoomEventView {
  return {
    envelope_id: `event-${seq}`,
    content: {
      event_id: `event-${seq}`,
      topic_id: 'kukuri:topic:demo',
      channel_id: null,
      room_id: room.room_id,
      peer_id: peerId,
      seq,
      sent_at: seq,
      event: {
        type: 'avatar_transform',
        transform: {
          room_id: room.room_id,
          peer_id: peerId,
          seq,
          position: [seq, 0, seq],
          rotation: [0, 90, 0],
          animation,
          sent_at: seq,
        },
      },
    },
    envelope: {},
    received_at: seq,
    source_peer: peerId,
  };
}

function renderPanel(api: DesktopApi, syncStatus?: SyncStatus) {
  return render(
    <MetaverseRoomPanel
      api={api}
      activeTopic='kukuri:topic:demo'
      activeComposeChannel={{ kind: 'public' }}
      rooms={[room]}
      syncStatus={syncStatus ?? {
        connected: true,
        delivery_state: 'Live',
        peer_count: 1,
        pending_events: 0,
        status_detail: 'connected',
        configured_peers: [],
        subscribed_topics: ['kukuri:topic:demo'],
        topic_diagnostics: [],
        local_author_pubkey: 'f'.repeat(64),
        discovery: {
          mode: 'seeded_dht',
          connect_mode: 'direct_only',
          env_locked: false,
          configured_seed_peer_ids: [],
          bootstrap_seed_peer_ids: [],
          manual_ticket_peer_ids: [],
          connected_peer_ids: [],
          docs_assist_peer_ids: [],
          blob_assist_peer_ids: [],
          local_endpoint_id: 'local-endpoint-a',
        },
      }}
      locale='en'
      onRefresh={vi.fn()}
    />
  );
}

describe('MetaverseRoomPanel animation sharing', () => {
  test('normalizes backend avatar animation states', async () => {
    const baseApi = createDesktopMockApi();
    const api: DesktopApi = {
      ...baseApi,
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([
        avatarEvent('idle-remote', 'idle', 1),
        avatarEvent('walk-remote', 'walk', 2),
        avatarEvent('sprint-remote', 'sprint', 3),
        avatarEvent('jump-remote', 'jump', 4),
        avatarEvent('sitting-remote', 'sitting', 5),
        avatarEvent('unknown-remote', 'dancing', 6),
      ]),
    };

    renderPanel(api);

    await waitFor(() => {
      expect(screen.getByText(/idle-rem:idle/)).toBeInTheDocument();
      expect(screen.getByText(/walk-rem:walk/)).toBeInTheDocument();
      expect(screen.getByText(/sprint-r:sprint/)).toBeInTheDocument();
      expect(screen.getByText(/jump-rem:jump/)).toBeInTheDocument();
      expect(screen.getByText(/sitting-:sitting/)).toBeInTheDocument();
      expect(screen.getByText(/unknown-:idle/)).toBeInTheDocument();
    });
  });

  test('publishes local transform animation state', async () => {
    const user = userEvent.setup();
    const baseApi = createDesktopMockApi();
    const publishMetaverseRoomEvent = vi.fn(baseApi.publishMetaverseRoomEvent);
    const api: DesktopApi = {
      ...baseApi,
      publishMetaverseRoomEvent,
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([]),
    };

    renderPanel(api);
    await user.click(screen.getByRole('button', { name: 'Emit sprint transform' }));

    await waitFor(() => {
      const transformCall = publishMetaverseRoomEvent.mock.calls.find(
        ([, , , , event]) => event.type === 'avatar_transform'
      );
      expect(transformCall?.[4]).toMatchObject({
        type: 'avatar_transform',
        transform: {
          animation: 'sprint',
        },
      });
    });
  });

  test('keeps room panel usable when animation playback falls back', async () => {
    const user = userEvent.setup();
    const api = createDesktopMockApi();

    renderPanel(api);
    await user.click(screen.getByRole('button', { name: 'Mark animation fallback' }));

    expect(screen.getByLabelText('Metaverse room viewport')).toBeInTheDocument();
    expect(screen.getByText(/fallback-primitive/)).toBeInTheDocument();
  });
});
