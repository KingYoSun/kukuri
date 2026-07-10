import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { GameRoomView } from '@/lib/api';
import { MetaverseRoomDiscovery } from './MetaverseRoomDiscovery';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

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

function renderDiscovery(
  overrides: Partial<Parameters<typeof MetaverseRoomDiscovery>[0]> = {}
) {
  const props: Parameters<typeof MetaverseRoomDiscovery>[0] = {
    rooms: [room],
    selectedRoomId: null,
    joinedRoomIds: new Set(),
    pending: false,
    error: null,
    locale: 'en',
    localAuthorPubkey: room.host_pubkey,
    localProfile: null,
    knownAuthorsByPubkey: {},
    mediaObjectUrls: {},
    onCreateRoom: vi.fn().mockResolvedValue(true),
    onJoinRoom: vi.fn(),
    ...overrides,
  };
  return { ...render(<MetaverseRoomDiscovery {...props} />), props };
}

describe('MetaverseRoomDiscovery', () => {
  test.each([
    ['en', 'Metaverse Rooms', 'Create metaverse room', 'Atrium', 'Join Room'],
    ['ja', 'メタバースルーム', 'メタバースルームを作成', 'アトリウム', 'ルームに参加'],
    ['zh-CN', '元宇宙房间', '创建元宇宙房间', '中庭', '加入房间'],
  ] as const)(
    'renders the primary discovery surface in %s',
    async (locale, title, createAction, titlePlaceholder, joinAction) => {
      await i18n.changeLanguage(locale);
      const user = userEvent.setup();
      renderDiscovery({ locale });

      expect(screen.getByRole('heading', { name: title })).toBeInTheDocument();
      await user.click(screen.getByRole('button', { name: createAction }));
      expect(screen.getByPlaceholderText(titlePlaceholder)).toBeInTheDocument();
      expect(screen.getByRole('button', { name: joinAction })).toBeInTheDocument();
    }
  );

  test('shows the empty state and an action error without an API dependency', () => {
    renderDiscovery({ rooms: [], error: 'Room action failed' });

    expect(screen.getByText('0 rooms in this topic')).toBeInTheDocument();
    expect(screen.getByText('No metaverse rooms in this topic.')).toBeInTheDocument();
    expect(screen.getByText('Room action failed')).toBeInTheDocument();
  });

  test('validates and normalizes the create-room callback input', async () => {
    const user = userEvent.setup();
    const onCreateRoom = vi.fn().mockResolvedValue(true);
    renderDiscovery({ rooms: [], onCreateRoom });

    await user.click(screen.getByRole('button', { name: 'Create metaverse room' }));
    await user.click(screen.getAllByRole('button', { name: 'Create metaverse room' })[1]);
    expect(screen.getByText('Room title is required')).toBeInTheDocument();
    expect(onCreateRoom).not.toHaveBeenCalled();

    await user.type(screen.getByPlaceholderText('Atrium'), '  Contract room  ');
    await user.type(screen.getByPlaceholderText('Small social space'), '  Contract space  ');
    const maxPeersInput = screen.getByLabelText('Max peers');
    await user.clear(maxPeersInput);
    await user.type(maxPeersInput, '12');
    await user.click(screen.getAllByRole('button', { name: 'Create metaverse room' })[1]);

    expect(onCreateRoom).toHaveBeenCalledWith({
      title: 'Contract room',
      description: 'Contract space',
      maxPeers: 12,
    });
    expect(screen.getByPlaceholderText('Atrium')).toHaveValue('');
    expect(maxPeersInput).toHaveValue('8');
  });

  test('preserves room card host fallback, selected/joined state, and Join callback', async () => {
    const user = userEvent.setup();
    const onJoinRoom = vi.fn();
    renderDiscovery({
      selectedRoomId: room.room_id,
      joinedRoomIds: new Set([room.room_id]),
      onJoinRoom,
    });

    expect(screen.getByText('1 room in this topic')).toBeInTheDocument();
    expect(screen.getByText(`Host: ${room.host_pubkey.slice(0, 10)}`)).toBeInTheDocument();
    expect(screen.getByText('Joined')).toBeInTheDocument();
    expect(screen.getByText(room.title).closest('article')).toHaveClass('metaverse-room-card-active');

    await user.click(screen.getByRole('button', { name: 'Join Room' }));
    expect(onJoinRoom).toHaveBeenCalledWith(room.room_id);
  });

  test('disables all create controls while an action is pending', async () => {
    const user = userEvent.setup();
    renderDiscovery({ rooms: [], pending: true });

    await user.click(screen.getByRole('button', { name: 'Create metaverse room' }));
    expect(screen.getByPlaceholderText('Atrium')).toBeDisabled();
    expect(screen.getByLabelText('Max peers')).toBeDisabled();
    expect(screen.getByPlaceholderText('Small social space')).toBeDisabled();
    expect(screen.getAllByRole('button', { name: 'Create metaverse room' })[1]).toBeDisabled();
  });
});
