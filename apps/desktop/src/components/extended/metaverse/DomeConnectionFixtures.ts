import type { DomeConnectionTopologyView, GameRoomView } from '@/lib/api';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';

export const connectionFixtureContext = { kind: 'topic' as const, topic_id: 'kukuri:topic:connections-review' };
export function connectionFixtureRoom(id: string): GameRoomView {
  return { room_id: id, host_pubkey: id.repeat(64), title: `Dome ${id.toUpperCase()}`, description: '', status: 'Waiting', scores: [],
    room_kind: 'metaverse_room', manifest_blob_hash: id, updated_at: 1, channel_id: null, audience_label: 'Public',
    metaverse: createDefaultMetaverseRoomState(8, { roomId: id, ownerPubkey: id.repeat(64), topicId: connectionFixtureContext.topic_id }) };
}
export const connectionFixtureRooms = ['a', 'b', 'c', 'd'].map(connectionFixtureRoom);
export function connectionFixtureTopology(): DomeConnectionTopologyView {
  const endpoint = (id: string, direction: 'east' | 'west' | 'north' | 'south') => ({ instance_id: id, instance_generation: 1, owner_pubkey: id.repeat(64), direction });
  return {
    proposals: [],
    connections: [['a', 'b'], ['b', 'c']].map(([a, b], i) => ({ record: {
      agreement: { connection_id: `${a}-${b}`, proposal_id: `proposal-${a}-${b}`, spatial_context: connectionFixtureContext,
        proposer: endpoint(a, i ? 'north' : 'east'), receiver: endpoint(b, i ? 'south' : 'west'), activation_generation: 1 },
      receiver_slot_generation: 1, observed_active_connection_ids: [], status: 'active', lifecycle_generation: 1,
      lifecycle_actor: null, lifecycle_reason: null, lifecycle_deadline_at: null,
    } })),
    resolution: { topology: { spatial_context: connectionFixtureContext, topology_digest: 'review', active_connection_ids: ['a-b', 'b-c'],
      components: [{ root_instance_id: 'a', instance_ids: ['a', 'b', 'c'], connection_ids: ['a-b', 'b-c'],
        coordinates_cm: { a: [0, 0, 0], b: [5700, 0, 0], c: [5700, 0, -5700] } },
      { root_instance_id: 'd', instance_ids: ['d'], connection_ids: [], coordinates_cm: { d: [0, 0, 0] } }] }, rejected_connections: [] },
  };
}
