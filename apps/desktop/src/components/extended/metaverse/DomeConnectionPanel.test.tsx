import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { DomeConnectionTopologyView, GameRoomView } from '@/lib/api';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';
import { DomeConnectionPanel } from './DomeConnectionPanel';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

const context = { kind: 'topic' as const, topic_id: 'kukuri:topic:connections-ui' };

function room(id: string, owner: string, title: string): GameRoomView {
  return {
    room_id: id,
    host_pubkey: owner,
    title,
    description: '',
    status: 'Waiting',
    phase_label: 'fixed-dome-v1',
    scores: [],
    room_kind: 'metaverse_room',
    metaverse: createDefaultMetaverseRoomState(8, {
      roomId: id,
      topicId: context.topic_id,
      ownerPubkey: owner,
    }),
    manifest_blob_hash: `manifest-${id}`,
    updated_at: 1,
    channel_id: null,
    audience_label: 'Public',
  };
}

const owner = 'a'.repeat(64);
const receiverOwner = 'b'.repeat(64);
const localRoom = room('dome-a', owner, 'Home Dome');
const receiverRoom = room('dome-b', receiverOwner, 'Neighbor Dome');

function emptyTopology(): DomeConnectionTopologyView {
  return {
    proposals: [],
    connections: [],
    resolution: {
      topology: {
        spatial_context: context,
        components: [localRoom, receiverRoom].map((item) => ({
          root_instance_id: item.room_id,
          instance_ids: [item.room_id],
          connection_ids: [],
          coordinates_cm: { [item.room_id]: [0, 0, 0] },
        })),
        active_connection_ids: [],
        topology_digest: 'empty-topology',
      },
      rejected_connections: [],
    },
  };
}

function actions(topology = emptyTopology()) {
  return {
    createRoom: vi.fn(),
    publishRoomEvent: vi.fn(),
    listRoomEvents: vi.fn(),
    importRoomAsset: vi.fn(),
    getBlobPreviewUrl: vi.fn(),
    updateRoom: vi.fn(),
    getHosting: vi.fn(),
    startOwnerHosting: vi.fn(),
    delegateHosting: vi.fn(),
    closeHosting: vi.fn(),
    submitSessionInput: vi.fn(),
    commitLayout: vi.fn(),
    resyncSnapshots: vi.fn(),
    moveRoom: vi.fn(),
    listConnections: vi.fn().mockResolvedValue(topology),
    createConnectionProposal: vi.fn().mockResolvedValue(topology.proposals[0]),
    acceptConnectionProposal: vi.fn(),
    withdrawConnectionProposal: vi.fn(),
    revokeConnection: vi.fn(),
    refresh: vi.fn(),
  } satisfies MetaverseRoomActions;
}

describe('DomeConnectionPanel', () => {
  test('renders four fixed direction slots and creates an opposite-slot proposal', async () => {
    const user = userEvent.setup();
    const roomActions = actions();
    render(
      <DomeConnectionPanel
        actions={roomActions}
        room={localRoom}
        rooms={[localRoom, receiverRoom]}
        localAuthorPubkey={owner}
        locale='en'
      />
    );

    await waitFor(() => expect(roomActions.listConnections).toHaveBeenCalledWith(context));
    for (const direction of ['North', 'East', 'South', 'West']) {
      expect(screen.getByRole('heading', { name: direction })).toBeInTheDocument();
    }
    const selects = screen.getAllByRole('combobox');
    await user.selectOptions(selects[1], receiverRoom.room_id);
    await user.click(screen.getAllByRole('button', { name: 'Propose Connection' })[1]);

    expect(roomActions.createConnectionProposal).toHaveBeenCalledWith(
      expect.stringMatching(/^dome-proposal-/),
      context,
      localRoom.room_id,
      receiverRoom.room_id,
      'east'
    );
  });
});
