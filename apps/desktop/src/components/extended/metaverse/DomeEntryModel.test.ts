import { describe, expect, it } from 'vitest';

import type { GameRoomView } from '@/lib/api';
import { resolveDomeEntryOrder } from './DomeEntryModel';

function room(id: string, owner: string, hosted = true): GameRoomView {
  return {
    room_id: id,
    host_pubkey: owner,
    title: id,
    description: '',
    status: 'Running',
    phase_label: null,
    scores: [],
    room_kind: 'metaverse_room',
    metaverse: {
      world_version: 2,
      instance_id: id,
      spatial_context: { kind: 'topic', topic_id: 'topic' },
      instance_generation: 1,
      instance_status: 'active',
      relationship_detach: null,
      replacement_instance_id: null,
      preset_ref: {
        preset_id: `preset-${id}`,
        owner_pubkey: owner,
        revision: 1,
        manifest_blob_hash: `hash-${id}`,
        manifest_mime: 'application/json',
        manifest_bytes: 1,
      },
      session_id: id,
      max_peers: 8,
      dome: { spec_id: 'kukuri:dome:fixed:v1', customization: { surface: { wall_material: 'concrete', floor_material: 'concrete', wall_texture: null, floor_texture: null }, environment: { key_light_milli: 2400, ambient_light_milli: 400, fog_density_micros: 8000, gravity_milli: 9800 }, persistent_props: [] } },
      default_spawn: { position: [0, 0, 260], rotation: [0, 180, 0] },
      asset_refs: [],
      chat_history: [],
    },
    dome_hosting: hosted ? { kind: 'owner_hosted' } : { kind: 'closed' },
    manifest_blob_hash: `hash-${id}`,
    updated_at: 1,
    channel_id: null,
    audience_label: 'Public',
  };
}

describe('resolveDomeEntryOrder', () => {
  it('orders own hosted, last visited, configured, then stable accessible list', () => {
    const rooms = [room('d', 'other'), room('c', 'other'), room('b', 'other'), room('a', 'me')];
    expect(resolveDomeEntryOrder({
      rooms,
      localAuthorPubkey: 'me',
      lastVisitedInstanceId: 'b',
      configuredEntryInstanceId: 'c',
    }).map((candidate) => candidate.room_id)).toEqual(['a', 'b', 'c', 'd']);
  });

  it('omits unavailable rooms and deduplicates priorities', () => {
    expect(resolveDomeEntryOrder({
      rooms: [room('a', 'me'), room('b', 'other', false)],
      localAuthorPubkey: 'me',
      lastVisitedInstanceId: 'a',
      configuredEntryInstanceId: 'b',
    }).map((candidate) => candidate.room_id)).toEqual(['a']);
  });
});
