import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, test, vi } from 'vitest';
import { useState, type ReactNode } from 'react';

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
    hud: ReactNode;
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
      {props.hud}
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

type RenderPanelOptions = {
  rooms?: GameRoomView[];
  syncStatus?: SyncStatus;
  onRefresh?: () => Promise<void>;
};

function createSyncStatus(): SyncStatus {
  return {
    connected: true,
    delivery_state: 'Live',
    peer_count: 1,
    pending_events: 0,
    status_detail: 'connected',
    configured_peers: [],
    subscribed_topics: ['kukuri:topic:demo'],
    active_path: 'direct_p2p',
    fallback_peer_ids: [],
    topic_diagnostics: [],
    local_author_pubkey: 'f'.repeat(64),
    discovery: {
      mode: 'seeded_dht' as const,
      connect_mode: 'direct_only' as const,
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
  };
}

function panelElement(api: DesktopApi, options: RenderPanelOptions = {}) {
  const effectiveSyncStatus = options.syncStatus ?? createSyncStatus();
  return (
    <MetaverseRoomPanel
      api={api}
      activeTopic='kukuri:topic:demo'
      activeComposeChannel={{ kind: 'public' }}
      rooms={options.rooms ?? [room]}
      syncStatus={effectiveSyncStatus}
      locale='en'
      localProfile={{
        pubkey: effectiveSyncStatus.local_author_pubkey,
        name: 'host',
        display_name: 'Host Author',
        about: null,
        picture: 'https://example.com/host.png',
        picture_asset: null,
        updated_at: 1,
      }}
      onRefresh={options.onRefresh ?? vi.fn()}
    />
  );
}

function renderPanel(api: DesktopApi, options: RenderPanelOptions = {}) {
  return render(panelElement(api, options));
}

describe('MetaverseRoomPanel animation sharing', () => {
  test('does not join an existing room until Join Room is clicked', async () => {
    const baseApi = createDesktopMockApi();
    const publishMetaverseRoomEvent = vi.fn(baseApi.publishMetaverseRoomEvent);
    const api: DesktopApi = {
      ...baseApi,
      publishMetaverseRoomEvent,
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([]),
    };

    renderPanel(api);

    expect(screen.queryByLabelText('Metaverse room viewport')).not.toBeInTheDocument();
    expect(publishMetaverseRoomEvent).not.toHaveBeenCalled();
  });

  test('joins a room from the explicit Join Room action', async () => {
    const user = userEvent.setup();
    const baseApi = createDesktopMockApi();
    const publishMetaverseRoomEvent = vi.fn(baseApi.publishMetaverseRoomEvent);
    const api: DesktopApi = {
      ...baseApi,
      publishMetaverseRoomEvent,
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([]),
    };

    renderPanel(api);
    await user.click(screen.getByRole('button', { name: 'Join Room' }));

    expect(screen.getByLabelText('Metaverse room viewport')).toBeInTheDocument();
    await waitFor(() => {
      expect(
        publishMetaverseRoomEvent.mock.calls.some(([, , , , event]) => event.type === 'presence_join')
      ).toBe(true);
    });
  });

  test('room HUD can be resized and collapsed', async () => {
    const user = userEvent.setup();
    const baseApi = createDesktopMockApi();
    const api: DesktopApi = {
      ...baseApi,
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([]),
    };

    renderPanel(api);
    await user.click(screen.getByRole('button', { name: 'Join Room' }));

    expect(screen.queryByText(/Topic: kukuri:topic:demo/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Debug details' })).toHaveAttribute(
      'aria-expanded',
      'false'
    );
    await user.click(screen.getByRole('button', { name: 'Debug details' }));
    expect(screen.getByText(/Topic: kukuri:topic:demo/)).toBeInTheDocument();

    expect(document.querySelector('.metaverse-room-hud')).toHaveAttribute('data-size', 'compact');
    await user.click(screen.getByRole('button', { name: 'Expand room HUD' }));
    expect(document.querySelector('.metaverse-room-hud')).toHaveAttribute('data-size', 'wide');
    expect(screen.getByRole('button', { name: 'Shrink room HUD' })).toBeInTheDocument();

    await user.click(screen.getByRole('button', { name: 'Hide room HUD' }));
    expect(document.querySelector('.metaverse-room-hud')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Open room HUD' }));
    expect(document.querySelector('.metaverse-room-hud')).toHaveAttribute('data-size', 'wide');
  });

  test('keeps create room controls collapsed until opened', async () => {
    const user = userEvent.setup();
    const api = createDesktopMockApi();

    renderPanel(api);

    expect(screen.queryByPlaceholderText('Atrium')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Create metaverse room' }));
    expect(screen.getByPlaceholderText('Atrium')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Small social space')).toBeInTheDocument();
  });

  test('opens the created room after refreshed rooms include it', async () => {
    const user = userEvent.setup();
    const baseApi = createDesktopMockApi();
    const createdRoom = {
      ...room,
      room_id: 'created-metaverse-room',
      title: 'Created room',
      manifest_blob_hash: 'mock-created-metaverse-room',
    };
    const api: DesktopApi = {
      ...baseApi,
      createMetaverseRoom: vi.fn().mockResolvedValue(createdRoom.room_id),
      publishMetaverseRoomEvent: vi.fn(baseApi.publishMetaverseRoomEvent),
      listMetaverseRoomEvents: vi.fn().mockResolvedValue([]),
    };
    function CreatedRoomHarness() {
      const [rooms, setRooms] = useState<GameRoomView[]>([]);
      return panelElement(api, {
        rooms,
        onRefresh: async () => {
          setRooms([createdRoom]);
        },
      });
    }

    render(<CreatedRoomHarness />);
    await user.click(screen.getByRole('button', { name: 'Create metaverse room' }));
    await user.type(screen.getByPlaceholderText('Atrium'), 'Created room');
    await user.click(screen.getAllByRole('button', { name: 'Create metaverse room' })[1]);

    expect(screen.getByLabelText('Metaverse room viewport')).toBeInTheDocument();
    await waitFor(() => {
      expect(api.publishMetaverseRoomEvent).toHaveBeenCalled();
    });
  });

  test('renders host identity from profile data', () => {
    const api = createDesktopMockApi();

    renderPanel(api);

    expect(screen.getByText('Host: Host Author')).toBeInTheDocument();
    expect(screen.getByText('Host: Host Author').previousElementSibling).toHaveAttribute(
      'data-avatar-src',
      'https://example.com/host.png'
    );
  });

  test('normalizes backend avatar animation states', async () => {
    const user = userEvent.setup();
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
    await user.click(screen.getByRole('button', { name: 'Join Room' }));
    await user.click(screen.getByRole('button', { name: 'Debug details' }));

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
    await user.click(screen.getByRole('button', { name: 'Join Room' }));
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
    await user.click(screen.getByRole('button', { name: 'Join Room' }));
    await user.click(screen.getByRole('button', { name: 'Mark animation fallback' }));
    await user.click(screen.getByRole('button', { name: 'Debug details' }));

    expect(screen.getByLabelText('Metaverse room viewport')).toBeInTheDocument();
    expect(screen.getByText(/fallback-primitive/)).toBeInTheDocument();
  });
});
