import { describe, expect, test } from 'vitest';
import { connectionMap, connectionRooms, connectionSlot } from './DomeConnectionModel';
import { connectionFixtureRooms as rooms, connectionFixtureTopology } from './DomeConnectionFixtures';

describe('confirmed component map', () => {
  test('centers the current Dome, preserves north and excludes other components', () => {
    const map = connectionMap(connectionFixtureTopology(), rooms[1], rooms);
    expect(map.nodes.map(n => [n.id, n.x, n.z])).toEqual([['a', -1, 0], ['b', 0, 0], ['c', 0, -1]]);
    expect(map.edges).toHaveLength(2);
  });
  test('does not name missing or foreign Context endpoints', () => {
    const foreign = structuredClone(rooms[2]);
    foreign.metaverse!.spatial_context = { kind: 'channel', topic_id: 'private', channel_id: 'secret' };
    const map = connectionMap(connectionFixtureTopology(), rooms[0], [rooms[0], rooms[1], foreign]);
    expect(map.nodes.find(n => n.id === 'c')?.room).toBeUndefined();
    expect(connectionRooms(rooms[0], [foreign])).toEqual([]);
    expect(map.nodes.find(n => n.id === 'a')?.current).toBe(true);
  });
  test('does not turn missing current topology into a confirmed open slot', () => {
    const topology = connectionFixtureTopology(); topology.connections = []; topology.resolution.topology.components = [];
    expect(connectionSlot(topology, rooms[0], 'north').status).toBe('unknown');
  });
  test('keeps draining geometry and shows a blocked terminal slot after split', () => {
    const topology = connectionFixtureTopology(); topology.connections[0].record.status = 'draining';
    expect(connectionSlot(topology, rooms[0], 'east').status).toBe('draining');
    expect(connectionMap(topology, rooms[0], rooms).edges).toHaveLength(2);
    topology.connections[0].record.status = 'revoked'; topology.connections[0].record.lifecycle_reason = 'owners_blocked';
    topology.resolution.topology.active_connection_ids = ['b-c'];
    topology.resolution.topology.components = [{ root_instance_id: 'a', instance_ids: ['a'], connection_ids: [], coordinates_cm: { a: [0, 0, 0] } }];
    expect(connectionSlot(topology, rooms[0], 'east').status).toBe('blocked');
    expect(connectionMap(topology, rooms[0], rooms).nodes.map(n => n.id)).toEqual(['a']);
  });
});
