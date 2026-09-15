import type { DomeConnectionEndpointV1, DomeConnectionTopologyView, DomeDirection, GameRoomView } from '@/lib/api';
import { connectionContextKey } from './useDomeConnections';

export const CONNECTION_DIRECTIONS: DomeDirection[] = ['north', 'east', 'south', 'west'];

export function connectionRooms(room: GameRoomView, rooms: GameRoomView[]) {
  const context = connectionContextKey(room.metaverse?.spatial_context);
  return rooms.filter(candidate => candidate.metaverse && connectionContextKey(candidate.metaverse.spatial_context) === context
    && candidate.metaverse.instance_status === 'active' && !candidate.metaverse.relationship_detach);
}

export function endpointIsCurrent(endpoint: DomeConnectionEndpointV1, room: GameRoomView) {
  return endpoint.instance_id === room.metaverse?.instance_id && endpoint.instance_generation === room.metaverse.instance_generation
    && endpoint.owner_pubkey === room.host_pubkey;
}

export function connectionSlot(topology: DomeConnectionTopologyView, room: GameRoomView, direction: DomeDirection) {
  const currentId = room.metaverse!.instance_id;
  const matching = topology.connections.filter(({ record }) => [record.agreement.proposer, record.agreement.receiver]
    .some(endpoint => endpoint.instance_id === currentId && endpoint.direction === direction));
  const effective = new Set(topology.resolution.topology.active_connection_ids);
  const connection = matching.find(({ record }) => effective.has(record.agreement.connection_id) && ['active', 'draining'].includes(record.status))
    ?? matching.find(({ record }) => record.status === 'accepted')
    ?? [...matching].sort((a, b) => b.record.lifecycle_generation - a.record.lifecycle_generation)[0];
  const proposals = topology.proposals.filter(({ proposal }) => [proposal.proposer, proposal.receiver]
    .some(endpoint => endpoint.instance_id === currentId && endpoint.direction === direction));
  const currentKnown = topology.resolution.topology.components.some(component => component.instance_ids.includes(currentId));
  const status = connection ? connection.record.status === 'revoked'
    ? connection.record.lifecycle_reason === 'owners_blocked' ? 'blocked' : 'closed'
    : connection.record.status
    : proposals.some(p => !['accepted', 'discarded'].includes(p.status)) ? 'proposed' : currentKnown ? 'open' : 'unknown';
  return { connection, proposals, status };
}

export function connectionMap(topology: DomeConnectionTopologyView, room: GameRoomView, rooms: GameRoomView[]) {
  const currentId = room.metaverse!.instance_id;
  const component = topology.resolution.topology.components.find(c => c.instance_ids.includes(currentId));
  const origin = component?.coordinates_cm[currentId] ?? [0, 0, 0];
  const known = new Map(connectionRooms(room, rooms).map(r => [r.metaverse!.instance_id, r]));
  known.set(currentId, room);
  const nodes = (component?.instance_ids ?? [currentId]).map(id => ({ id, room: known.get(id), current: id === currentId,
    x: ((component?.coordinates_cm[id]?.[0] ?? origin[0]) - origin[0]) / 5700,
    z: ((component?.coordinates_cm[id]?.[2] ?? origin[2]) - origin[2]) / 5700 }));
  const ids = new Set(nodes.map(n => n.id));
  const edges = topology.connections.filter(c => component?.connection_ids.includes(c.record.agreement.connection_id)
    && ids.has(c.record.agreement.proposer.instance_id) && ids.has(c.record.agreement.receiver.instance_id));
  return { nodes, edges };
}
