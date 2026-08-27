import type {
  DomeCustomizationV1,
  DomeDirection,
  DomeEnvironmentV1,
  MetaverseColliderV1,
  MetaverseInteractionKind,
  MetaversePersistentPropV1,
  MetaverseRoomStateV1,
  SharedRoomObjectV1,
} from '@/lib/api';

export const FIXED_DOME_SPEC_ID = 'fixed_dome_v1';
export const METAVERSE_WORLD_VERSION = 3;
export const DOME_INNER_RADIUS_CM = 2_000;
export const DOME_OUTER_RADIUS_CM = 2_200;
export const DOME_APEX_HEIGHT_CM = 2_000;
export const DOME_WALL_THICKNESS_CM = 200;
export const DOME_OPENING_WIDTH_CM = 500;
export const DOME_OPENING_HEIGHT_CM = 1_000;
export const DOME_OPENING_ARCH_RADIUS_CM = 250;
export const DOME_OPENING_RECT_HEIGHT_CM = 750;
export const DOME_CONNECTION_ZONE_DEPTH_CM = 1_500;
export const DOME_CONNECTION_BOUNDARY_OFFSET_CM = 2_850;
export const DOME_ADJACENT_CENTER_DISTANCE_CM = 5_700;

export const DOME_DIRECTIONS: DomeDirection[] = ['north', 'east', 'south', 'west'];
export const DOME_INTERACTIONS: MetaverseInteractionKind[] = ['grab', 'throw', 'push', 'sit'];

export type DomeBounds = {
  min: [number, number, number];
  max: [number, number, number];
};

export type DomeInteractionInput = {
  type: MetaverseInteractionKind;
  propId: string;
  actorPeerId: string;
  issuedAt: number;
};

export function createDefaultDomeCustomization(): DomeCustomizationV1 {
  return {
    surface: {
      wall_material: 'concrete',
      floor_material: 'stone',
      wall_texture: null,
      floor_texture: null,
    },
    environment: {
      key_light_milli: 2_400,
      ambient_light_milli: 400,
      fog_density_micros: 8_000,
      gravity_milli: 9_800,
    },
    persistent_props: [
      {
        prop_id: 'dome-prop-1',
        asset_ref: null,
        primitive_fallback: 'cube',
        position: [0, 50, -240],
        rotation: [0, 0, 0],
        scale: [100, 100, 100],
        visual_only: false,
        interactions: [...DOME_INTERACTIONS],
        collider: null,
      },
    ],
  };
}

export function createDefaultMetaverseRoomState(
  maxPeers: number | null = null,
  identity: { roomId?: string; topicId?: string; channelId?: string | null; ownerPubkey?: string } = {}
): MetaverseRoomStateV1 {
  const roomId = identity.roomId ?? 'dome-local';
  const topicId = identity.topicId ?? 'kukuri:topic:metaverse';
  const ownerPubkey = identity.ownerPubkey ?? 'local-owner';
  return {
    world_version: METAVERSE_WORLD_VERSION,
    instance_id: roomId,
    spatial_context: identity.channelId
      ? { kind: 'channel', topic_id: topicId, channel_id: identity.channelId }
      : { kind: 'topic', topic_id: topicId },
    instance_generation: 1,
    instance_status: 'active',
    relationship_detach: null,
    replacement_instance_id: null,
    preset_ref: {
      preset_id: `preset-${roomId}`,
      owner_pubkey: ownerPubkey,
      manifest_blob_hash: `mock-preset-${roomId}`,
      manifest_mime: 'application/vnd.kukuri.dome-preset+json',
      manifest_bytes: 1,
    },
    session_id: roomId,
    max_peers: maxPeers,
    dome: {
      spec_id: FIXED_DOME_SPEC_ID,
      customization: createDefaultDomeCustomization(),
    },
    default_spawn: {
      position: [0, 0, 260],
      rotation: [0, 180, 0],
    },
    asset_refs: [],
    chat_history: [],
  };
}

export function isDomeCustomizationValid(customization: DomeCustomizationV1): boolean {
  const { environment, persistent_props: props, surface } = customization;
  if (surface.wall_texture?.kind !== undefined && surface.wall_texture.kind !== 'texture') return false;
  if (surface.floor_texture?.kind !== undefined && surface.floor_texture.kind !== 'texture') return false;
  if (environment.key_light_milli < 0 || environment.key_light_milli > 4_000) return false;
  if (environment.ambient_light_milli < 0 || environment.ambient_light_milli > 2_000) return false;
  if (environment.fog_density_micros < 0 || environment.fog_density_micros > 200_000) return false;
  if (environment.gravity_milli < 1_000 || environment.gravity_milli > 30_000) return false;
  if (props.length > 64) return false;
  const ids = new Set<string>();
  return props.every((prop) => {
    if (!prop.prop_id.trim() || ids.has(prop.prop_id)) return false;
    ids.add(prop.prop_id);
    if (prop.scale.some((scale) => scale < 1 || scale > 1_000)) return false;
    const [x, y, z] = prop.position;
    if (y < 0 || y > DOME_APEX_HEIGHT_CM || x * x + y * y + z * z > DOME_INNER_RADIUS_CM ** 2) {
      return false;
    }
    if (new Set(prop.interactions).size !== prop.interactions.length) return false;
    return !prop.visual_only || prop.interactions.length === 0;
  });
}

export function persistentPropAsSharedObject(
  prop: MetaversePersistentPropV1 | null | undefined,
  updatedBy = '',
  updatedAt = 0
): SharedRoomObjectV1 {
  const fallback = createDefaultDomeCustomization().persistent_props[0];
  const source = prop ?? fallback;
  return {
    object_id: source.prop_id,
    asset_ref: source.asset_ref ?? null,
    primitive_fallback: source.primitive_fallback,
    position: [...source.position],
    rotation: [...source.rotation],
    scale: [...source.scale],
    updated_by: updatedBy,
    updated_at: updatedAt,
  };
}

export function interpolateDomeEnvironment(
  from: DomeEnvironmentV1,
  to: DomeEnvironmentV1,
  progress: number
): DomeEnvironmentV1 {
  const normalized = Math.min(1, Math.max(0, progress));
  const interpolate = (left: number, right: number) => Math.round(left + (right - left) * normalized);
  return {
    key_light_milli: interpolate(from.key_light_milli, to.key_light_milli),
    ambient_light_milli: interpolate(from.ambient_light_milli, to.ambient_light_milli),
    fog_density_micros: interpolate(from.fog_density_micros, to.fog_density_micros),
    gravity_milli: interpolate(from.gravity_milli, to.gravity_milli),
  };
}

export function resolveDomeCollider(
  explicit: MetaverseColliderV1 | null | undefined,
  bounds: DomeBounds
): MetaverseColliderV1 {
  if (explicit) return explicit;
  const width = bounds.max[0] - bounds.min[0];
  const height = bounds.max[1] - bounds.min[1];
  const depth = bounds.max[2] - bounds.min[2];
  if (width <= 0 || height <= 0 || depth <= 0) {
    throw new Error('Dome collider bounds must have positive volume');
  }
  const radius = Math.ceil(Math.max(width, depth) / 2);
  return {
    shape: 'capsule',
    center: [
      Math.round((bounds.min[0] + bounds.max[0]) / 2),
      Math.round((bounds.min[1] + bounds.max[1]) / 2),
      Math.round((bounds.min[2] + bounds.max[2]) / 2),
    ],
    radius,
    half_height: Math.max(0, Math.ceil(height / 2) - radius),
  };
}

export function domeDirectionOffset(direction: DomeDirection): [number, number, number] {
  if (direction === 'north') return [0, 0, -DOME_ADJACENT_CENTER_DISTANCE_CM];
  if (direction === 'east') return [DOME_ADJACENT_CENTER_DISTANCE_CM, 0, 0];
  if (direction === 'south') return [0, 0, DOME_ADJACENT_CENTER_DISTANCE_CM];
  return [-DOME_ADJACENT_CENTER_DISTANCE_CM, 0, 0];
}

export function openingContains(tangentCm: number, heightCm: number): boolean {
  const tangent = Math.abs(tangentCm);
  if (heightCm < 0 || heightCm > DOME_OPENING_HEIGHT_CM) return false;
  if (heightCm <= DOME_OPENING_RECT_HEIGHT_CM) return tangent <= DOME_OPENING_WIDTH_CM / 2;
  const archY = heightCm - DOME_OPENING_RECT_HEIGHT_CM;
  return tangent * tangent + archY * archY <= DOME_OPENING_ARCH_RADIUS_CM ** 2;
}

export function clampAvatarToDome(position: [number, number, number]): [number, number, number] {
  let [x, y, z] = position;
  y = Math.max(0, Math.min(DOME_APEX_HEIGHT_CM, y));
  const radialDistance = Math.hypot(x, z);

  const alongNorthSouth = radialDistance > DOME_INNER_RADIUS_CM
    && Math.abs(z) >= Math.abs(x)
    && openingContains(x, y);
  const alongEastWest = radialDistance > DOME_INNER_RADIUS_CM
    && Math.abs(x) > Math.abs(z)
    && openingContains(z, y);
  if (alongNorthSouth || alongEastWest) {
    const limit = DOME_CONNECTION_BOUNDARY_OFFSET_CM;
    if (alongNorthSouth) z = Math.max(-limit, Math.min(limit, z));
    if (alongEastWest) x = Math.max(-limit, Math.min(limit, x));
    return [Math.round(x), Math.round(y), Math.round(z)];
  }

  const horizontalLimit = Math.sqrt(Math.max(0, DOME_INNER_RADIUS_CM ** 2 - y ** 2));
  if (radialDistance <= horizontalLimit) return [Math.round(x), Math.round(y), Math.round(z)];
  if (radialDistance === 0) return [0, Math.round(y), 0];
  const scale = horizontalLimit / radialDistance;
  return [Math.round(x * scale), Math.round(y), Math.round(z * scale)];
}

export function createDomeInteractionInput(
  type: MetaverseInteractionKind,
  propId: string,
  actorPeerId: string,
  issuedAt = Date.now()
): DomeInteractionInput {
  return { type, propId, actorPeerId, issuedAt };
}
