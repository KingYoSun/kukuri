import type {
  DomeBoundaryStateV1,
  DomeConnectionTopologyView,
  DomeDirection,
  DomeEnvironmentV1,
  DomeHostingView,
  GameRoomView,
} from '@/lib/api';

import {
  DOME_APEX_HEIGHT_CM,
  DOME_CONNECTION_BOUNDARY_OFFSET_CM,
  DOME_CONNECTION_ZONE_DEPTH_CM,
  DOME_DIRECTIONS,
  DOME_INNER_RADIUS_CM,
  openingContains,
} from './DomeSceneModel';

export const DOME_TRANSITION_CROSSING_HYSTERESIS_CM = 10;

export type DomeNeighborTransitionView = {
  connectionId: string;
  topologyDigest: string;
  direction: DomeDirection;
  targetDirection: DomeDirection;
  room: GameRoomView;
  relativeCoordinateCm: [number, number, number];
  boundaryState: DomeBoundaryStateV1;
  textureUrls: { wall: string | null; floor: string | null };
};

const directionOrder = new Map(DOME_DIRECTIONS.map((direction, index) => [direction, index]));

export function oppositeDomeDirection(direction: DomeDirection): DomeDirection {
  if (direction === 'north') return 'south';
  if (direction === 'east') return 'west';
  if (direction === 'south') return 'north';
  return 'east';
}

function hostingBoundaryState(
  room: GameRoomView,
  hosting: DomeHostingView | undefined
): DomeBoundaryStateV1 {
  if (!hosting) return 'loading';
  if (hosting.state.kind === 'grace_period' || hosting.state.kind === 'transferring') return 'stale';
  if (
    (hosting.state.kind !== 'owner_hosted' && hosting.state.kind !== 'community_node_hosted') ||
    !hosting.state.session_id
  ) return 'unhosted';
  const capacity = room.metaverse?.max_peers;
  if (capacity !== null && capacity !== undefined && hosting.participants >= capacity) return 'full';
  return 'ready';
}

export function resolveActiveDomeNeighbors(
  topology: DomeConnectionTopologyView,
  currentRoom: GameRoomView,
  rooms: GameRoomView[],
  hostingByInstance: Record<string, DomeHostingView | undefined>,
  assetStateByInstance: Record<string, 'loading' | 'ready' | 'error'> = {},
  textureUrlsByInstance: Record<string, { wall: string | null; floor: string | null }> = {}
): DomeNeighborTransitionView[] {
  const currentId = currentRoom.metaverse?.instance_id;
  if (!currentId) return [];
  const component = topology.resolution.topology.components.find((candidate) =>
    candidate.instance_ids.includes(currentId)
  );
  const currentCoordinate = component?.coordinates_cm[currentId];
  if (!component || !currentCoordinate) return [];
  const activeIds = new Set(topology.resolution.topology.active_connection_ids);
  const byInstance = new Map(
    rooms
      .filter((room) => room.metaverse)
      .map((room) => [room.metaverse!.instance_id, room])
  );

  return topology.connections
    .filter(({ record }) => record.status === 'active' && activeIds.has(record.agreement.connection_id))
    .flatMap(({ record }) => {
      const { proposer, receiver, connection_id: connectionId } = record.agreement;
      const source = proposer.instance_id === currentId
        ? proposer
        : receiver.instance_id === currentId
          ? receiver
          : null;
      if (!source) return [];
      const target = source === proposer ? receiver : proposer;
      const room = byInstance.get(target.instance_id);
      const coordinate = component.coordinates_cm[target.instance_id];
      if (!room || !coordinate) return [];
      let boundaryState = hostingBoundaryState(room, hostingByInstance[target.instance_id]);
      const assetState = assetStateByInstance[target.instance_id];
      if (boundaryState === 'ready' && assetState === 'loading') boundaryState = 'loading';
      if (boundaryState === 'ready' && assetState === 'error') boundaryState = 'error';
      return [{
        connectionId,
        topologyDigest: topology.resolution.topology.topology_digest,
        direction: source.direction,
        targetDirection: target.direction,
        room,
        relativeCoordinateCm: [
          coordinate[0] - currentCoordinate[0],
          coordinate[1] - currentCoordinate[1],
          coordinate[2] - currentCoordinate[2],
        ],
        boundaryState,
        textureUrls: textureUrlsByInstance[target.instance_id] ?? { wall: null, floor: null },
      } satisfies DomeNeighborTransitionView];
    })
    .sort((left, right) =>
      (directionOrder.get(left.direction) ?? 0) - (directionOrder.get(right.direction) ?? 0)
    )
    .slice(0, 4);
}

export function domeTransitionAxisCm(
  position: [number, number, number],
  direction: DomeDirection
): number {
  if (direction === 'north') return -position[2];
  if (direction === 'east') return position[0];
  if (direction === 'south') return position[2];
  return -position[0];
}

function directionAndTangent(position: [number, number, number]): {
  direction: DomeDirection;
  tangent: number;
} {
  const [x, , z] = position;
  if (Math.abs(z) >= Math.abs(x)) {
    return { direction: z < 0 ? 'north' : 'south', tangent: x };
  }
  return { direction: x < 0 ? 'west' : 'east', tangent: z };
}

export function clampAvatarToTransitionBoundaries(
  position: [number, number, number],
  boundaries: Partial<Record<DomeDirection, DomeBoundaryStateV1>>
): [number, number, number] {
  let [x, y, z] = position;
  y = Math.max(0, Math.min(DOME_APEX_HEIGHT_CM, y));
  const radialDistance = Math.hypot(x, z);
  const { direction, tangent } = directionAndTangent([x, y, z]);
  const state = boundaries[direction] ?? 'closed';
  if (
    radialDistance > DOME_INNER_RADIUS_CM &&
    state !== 'closed' &&
    openingContains(tangent, y)
  ) {
    const readyLimit = DOME_CONNECTION_BOUNDARY_OFFSET_CM + DOME_CONNECTION_ZONE_DEPTH_CM / 2;
    const closedLimit = DOME_CONNECTION_BOUNDARY_OFFSET_CM - DOME_TRANSITION_CROSSING_HYSTERESIS_CM;
    const limit = state === 'ready' ? readyLimit : closedLimit;
    if (direction === 'north') z = Math.max(-limit, z);
    if (direction === 'south') z = Math.min(limit, z);
    if (direction === 'east') x = Math.min(limit, x);
    if (direction === 'west') x = Math.max(-limit, x);
    return [Math.round(x), Math.round(y), Math.round(z)];
  }
  const horizontalLimit = Math.sqrt(Math.max(0, DOME_INNER_RADIUS_CM ** 2 - y ** 2));
  if (radialDistance <= horizontalLimit) return [Math.round(x), Math.round(y), Math.round(z)];
  if (radialDistance === 0) return [0, Math.round(y), 0];
  const scale = horizontalLimit / radialDistance;
  return [Math.round(x * scale), Math.round(y), Math.round(z * scale)];
}

export function domeTransitionProgress(
  position: [number, number, number],
  direction: DomeDirection
): number {
  const start = DOME_CONNECTION_BOUNDARY_OFFSET_CM - DOME_CONNECTION_ZONE_DEPTH_CM / 2;
  return Math.min(
    1,
    Math.max(0, (domeTransitionAxisCm(position, direction) - start) / DOME_CONNECTION_ZONE_DEPTH_CM)
  );
}

export function transitionNeighborAtPosition(
  previous: [number, number, number] | null,
  current: [number, number, number],
  neighbors: DomeNeighborTransitionView[]
): DomeNeighborTransitionView | null {
  if (!previous) return null;
  const { direction, tangent } = directionAndTangent(current);
  const neighbor = neighbors.find(
    (candidate) => candidate.direction === direction && candidate.boundaryState === 'ready'
  );
  if (!neighbor || !openingContains(tangent, current[1])) return null;
  const center = DOME_CONNECTION_BOUNDARY_OFFSET_CM;
  return domeTransitionAxisCm(previous, direction) <= center - DOME_TRANSITION_CROSSING_HYSTERESIS_CM &&
    domeTransitionAxisCm(current, direction) >= center + DOME_TRANSITION_CROSSING_HYSTERESIS_CM
    ? neighbor
    : null;
}

export function transitionNeighborInZone(
  position: [number, number, number],
  neighbors: DomeNeighborTransitionView[]
): DomeNeighborTransitionView | null {
  const { direction, tangent } = directionAndTangent(position);
  const neighbor = neighbors.find(
    (candidate) => candidate.direction === direction && candidate.boundaryState === 'ready'
  );
  if (!neighbor || !openingContains(tangent, position[1])) return null;
  const progress = domeTransitionProgress(position, direction);
  return progress > 0 && progress < 1 ? neighbor : null;
}

export function transformAvatarBetweenDomes(
  position: [number, number, number],
  targetCoordinate: [number, number, number]
): [number, number, number] {
  return [
    position[0] - targetCoordinate[0],
    position[1] - targetCoordinate[1],
    position[2] - targetCoordinate[2],
  ];
}

export function transitionEnvironmentAtPosition(
  position: [number, number, number],
  current: DomeEnvironmentV1,
  neighbors: DomeNeighborTransitionView[]
): { environment: DomeEnvironmentV1; direction: DomeDirection | null; progress: number } {
  const { direction, tangent } = directionAndTangent(position);
  const neighbor = neighbors.find((candidate) => candidate.direction === direction);
  const progress = domeTransitionProgress(position, direction);
  if (!neighbor || progress <= 0 || !openingContains(tangent, position[1])) {
    return { environment: current, direction: null, progress: 0 };
  }
  const target = neighbor.room.metaverse?.dome.customization.environment ?? current;
  const interpolate = (left: number, right: number) => Math.round(left + (right - left) * progress);
  return {
    environment: {
      key_light_milli: interpolate(current.key_light_milli, target.key_light_milli),
      ambient_light_milli: interpolate(current.ambient_light_milli, target.ambient_light_milli),
      fog_density_micros: interpolate(current.fog_density_micros, target.fog_density_micros),
      gravity_milli: interpolate(current.gravity_milli, target.gravity_milli),
    },
    direction,
    progress,
  };
}
