import { describe, expect, it } from 'vitest';

import type { DomeConnectionTopologyView, DomeHostingView, GameRoomView } from '@/lib/api';
import { createDefaultMetaverseRoomState } from './DomeSceneModel';
import {
  clampAvatarToTransitionBoundaries,
  domeTransitionProgress,
  resolveActiveDomeNeighbors,
  transitionNeighborAtPosition,
  transformAvatarBetweenDomes,
} from './DomeTransitionModel';

function room(instanceId: string): GameRoomView {
  return {
    room_id: instanceId,
    host_pubkey: instanceId.padEnd(64, 'a').slice(0, 64),
    title: instanceId,
    description: '',
    status: 'Waiting',
    phase_label: 'fixed-dome-v1',
    scores: [],
    room_kind: 'metaverse_room',
    metaverse: createDefaultMetaverseRoomState(8, { roomId: instanceId }),
    manifest_blob_hash: `manifest-${instanceId}`,
    updated_at: 1,
    channel_id: null,
    audience_label: 'Public',
  };
}

function hosting(instanceId: string, participants = 0): DomeHostingView {
  return {
    instance_id: instanceId,
    state: {
      kind: 'owner_hosted',
      host: null,
      lease_id: 'lease-1',
      lease_epoch: 1,
      lease_expires_at: 10_000,
      session_id: 'session-1',
      reason: null,
      last_heartbeat_at: 1,
    },
    lease: null,
    signed_lease_json: null,
    signed_activation_json: null,
    signed_close_json: null,
    instance_manifest_json: '{}',
    preset_manifest_json: '{}',
    participants,
    sleeping: false,
    resource_budget: {} as DomeHostingView['resource_budget'],
    resource_metrics: {} as DomeHostingView['resource_metrics'],
  };
}

function topology(): DomeConnectionTopologyView {
  const owner = 'a'.repeat(64);
  return {
    proposals: [],
    connections: [{
      record: {
        agreement: {
          connection_id: 'connection-1',
          proposal_id: 'proposal-1',
          spatial_context: { kind: 'topic', topic_id: 'kukuri:topic:metaverse' },
          proposer: { instance_id: 'dome-a', instance_generation: 1, owner_pubkey: owner, direction: 'north' },
          receiver: { instance_id: 'dome-b', instance_generation: 1, owner_pubkey: owner, direction: 'south' },
          activation_generation: 1,
        },
        receiver_slot_generation: 1,
        observed_active_connection_ids: [],
        status: 'active',
        lifecycle_generation: 1,
        lifecycle_actor: null,
        lifecycle_reason: null,
        lifecycle_deadline_at: null,
      },
    }],
    resolution: {
      topology: {
        spatial_context: { kind: 'topic', topic_id: 'kukuri:topic:metaverse' },
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
  };
}

describe('DomeTransitionModel', () => {
  it('active topology and hosting state resolve one ready neighbor', () => {
    const current = room('dome-a');
    const target = room('dome-b');
    const neighbors = resolveActiveDomeNeighbors(
      topology(),
      current,
      [current, target],
      { 'dome-b': hosting('dome-b') },
      { 'dome-b': 'ready' }
    );
    expect(neighbors).toMatchObject([{
      connectionId: 'connection-1',
      direction: 'north',
      targetDirection: 'south',
      relativeCoordinateCm: [0, 0, -5_700],
      boundaryState: 'ready',
    }]);
  });

  it('capacity and unavailable host close only the corresponding boundary', () => {
    const current = room('dome-a');
    const target = room('dome-b');
    expect(resolveActiveDomeNeighbors(topology(), current, [current, target], {
      'dome-b': hosting('dome-b', 8),
    }, { 'dome-b': 'ready' })[0].boundaryState).toBe('full');
    expect(resolveActiveDomeNeighbors(topology(), current, [current, target], {}, {
      'dome-b': 'ready',
    })[0].boundaryState).toBe('loading');
  });

  it('distinguishes offline, draining, blocked, and closed boundaries', () => {
    const current = room('dome-a');
    const target = room('dome-b');
    const offline = hosting('dome-b');
    offline.state.kind = 'grace_period';
    expect(resolveActiveDomeNeighbors(topology(), current, [current, target], {
      'dome-b': offline,
    })[0].boundaryState).toBe('offline');

    const draining = topology();
    draining.connections[0].record.status = 'draining';
    draining.connections[0].record.lifecycle_actor = 'a'.repeat(64);
    draining.connections[0].record.lifecycle_reason = 'owner_revoked';
    draining.connections[0].record.lifecycle_deadline_at = 4_000;
    expect(resolveActiveDomeNeighbors(draining, current, [current, target], {
      'dome-b': hosting('dome-b'),
    })[0].boundaryState).toBe('draining');

    const blocked = topology();
    blocked.connections[0].record.status = 'revoked';
    blocked.connections[0].record.lifecycle_actor = 'a'.repeat(64);
    blocked.connections[0].record.lifecycle_reason = 'owners_blocked';
    blocked.resolution.topology.active_connection_ids = [];
    blocked.resolution.topology.components = [];
    expect(resolveActiveDomeNeighbors(blocked, current, [current, target], {})[0].boundaryState)
      .toBe('blocked');

    blocked.connections[0].record.lifecycle_reason = 'owner_revoked';
    expect(resolveActiveDomeNeighbors(blocked, current, [current, target], {})[0].boundaryState)
      .toBe('closed');
  });

  it('loading boundary stops before center while ready boundary reaches the far end', () => {
    expect(clampAvatarToTransitionBoundaries([0, 0, -3_000], { north: 'loading' }))
      .toEqual([0, 0, -2_840]);
    expect(clampAvatarToTransitionBoundaries([0, 0, -3_700], { north: 'ready' }))
      .toEqual([0, 0, -3_600]);
    expect(clampAvatarToTransitionBoundaries([3_000, 0, 0], {}))
      .toEqual([2_000, 0, 0]);
  });

  it('crosses once in the travel direction and preserves component position', () => {
    const current = room('dome-a');
    const target = room('dome-b');
    const neighbors = resolveActiveDomeNeighbors(
      topology(), current, [current, target], { 'dome-b': hosting('dome-b') }, { 'dome-b': 'ready' }
    );
    expect(transitionNeighborAtPosition([0, 0, -2_830], [0, 0, -2_870], neighbors)?.room.room_id)
      .toBe('dome-b');
    expect(transitionNeighborAtPosition([0, 0, -2_870], [0, 0, -2_830], neighbors)).toBeNull();
    expect(transformAvatarBetweenDomes([0, 90, -2_870], [0, 0, -5_700]))
      .toEqual([0, 90, 2_830]);
    expect(domeTransitionProgress([0, 0, -2_850], 'north')).toBe(0.5);
  });
});
