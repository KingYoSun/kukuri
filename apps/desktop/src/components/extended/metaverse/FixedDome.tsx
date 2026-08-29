import { useEffect, useMemo, useState } from 'react';
import * as THREE from 'three';

import type {
  DomeBoundaryStateV1,
  DomeCustomizationV1,
  DomeDirection,
  DomeMaterialPreset,
} from '@/lib/api';
import {
  DOME_CONNECTION_BOUNDARY_OFFSET_CM,
  DOME_CONNECTION_ZONE_DEPTH_CM,
  DOME_DIRECTIONS,
  DOME_INNER_RADIUS_CM,
  DOME_OPENING_ARCH_RADIUS_CM,
  DOME_OPENING_RECT_HEIGHT_CM,
  DOME_OPENING_WIDTH_CM,
  DOME_OUTER_RADIUS_CM,
} from './DomeSceneModel';
import { createDomeHemisphereGeometry } from './DomeGeometry';

const SCENE_UNITS_PER_CENTIMETER = 0.01;

function materialColor(preset: DomeMaterialPreset): number {
  if (preset === 'metal') return 0x6f7f8b;
  if (preset === 'wood') return 0x7a5840;
  if (preset === 'stone') return 0x515a5d;
  return 0x59626f;
}

function useDomeTexture(url: string | null): THREE.Texture | null {
  const [texture, setTexture] = useState<THREE.Texture | null>(null);
  useEffect(() => {
    let cancelled = false;
    let loaded: THREE.Texture | null = null;
    setTexture(null);
    if (!url) return;
    new THREE.TextureLoader().load(
      url,
      (next) => {
        loaded = next;
        next.colorSpace = THREE.SRGBColorSpace;
        next.wrapS = THREE.RepeatWrapping;
        next.wrapT = THREE.RepeatWrapping;
        next.repeat.set(6, 3);
        if (!cancelled) setTexture(next);
      },
      undefined,
      () => {
        if (!cancelled) setTexture(null);
      }
    );
    return () => {
      cancelled = true;
      loaded?.dispose();
    };
  }, [url]);
  return texture;
}

function directionTransform(direction: DomeDirection): {
  position: [number, number, number];
  rotationY: number;
} {
  const boundary = DOME_CONNECTION_BOUNDARY_OFFSET_CM * SCENE_UNITS_PER_CENTIMETER;
  if (direction === 'north') return { position: [0, 0, -boundary], rotationY: 0 };
  if (direction === 'east') return { position: [boundary, 0, 0], rotationY: Math.PI / 2 };
  if (direction === 'south') return { position: [0, 0, boundary], rotationY: 0 };
  return { position: [-boundary, 0, 0], rotationY: Math.PI / 2 };
}

function boundaryColor(state: DomeBoundaryStateV1): number {
  if (state === 'offline') return 0xd69e2e;
  if (state === 'draining') return 0xcf6f2e;
  if (state === 'blocked') return 0xb42335;
  if (state === 'loading') return 0xd69e2e;
  if (state === 'full') return 0xc47f17;
  if (state === 'denied') return 0xb43b47;
  if (state === 'stale') return 0x7256a8;
  if (state === 'error') return 0xa7323e;
  return 0x59626f;
}

function ConnectionZone({
  direction,
  color,
  boundaryState,
}: {
  direction: DomeDirection;
  color: number;
  boundaryState: DomeBoundaryStateV1;
}) {
  const { position, rotationY } = directionTransform(direction);
  const width = DOME_OPENING_WIDTH_CM * SCENE_UNITS_PER_CENTIMETER;
  const wallHeight = DOME_OPENING_RECT_HEIGHT_CM * SCENE_UNITS_PER_CENTIMETER;
  const roofRadius = DOME_OPENING_ARCH_RADIUS_CM * SCENE_UNITS_PER_CENTIMETER;
  const depth = DOME_CONNECTION_ZONE_DEPTH_CM * SCENE_UNITS_PER_CENTIMETER;
  const roofSegments = 12;
  const roofStripWidth = Math.PI * roofRadius / roofSegments;

  return (
    <group position={position} rotation={[0, rotationY, 0]} userData={{ domeDirection: direction }}>
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, 0, 0]} receiveShadow>
        <planeGeometry args={[width, depth]} />
        <meshStandardMaterial color={color} roughness={0.9} side={THREE.DoubleSide} />
      </mesh>
      {[-width / 2, width / 2].map((x) => (
        <mesh key={x} position={[x, wallHeight / 2, 0]} receiveShadow>
          <boxGeometry args={[0.12, wallHeight, depth]} />
          <meshStandardMaterial color={color} roughness={0.82} />
        </mesh>
      ))}
      {Array.from({ length: roofSegments }, (_, index) => {
        const angle = ((index + 0.5) / roofSegments) * Math.PI;
        return (
          <mesh
            key={index}
            position={[
              Math.cos(angle) * roofRadius,
              wallHeight + Math.sin(angle) * roofRadius,
              0,
            ]}
            rotation={[0, 0, angle - Math.PI / 2]}
          >
            <boxGeometry args={[roofStripWidth, 0.12, depth]} />
            <meshStandardMaterial color={color} roughness={0.82} />
          </mesh>
        );
      })}
      <group position={[0, 0, 0]} userData={{ transitionCenter: true }}>
        <mesh position={[-width / 2, wallHeight / 2, 0]}>
          <boxGeometry args={[0.05, wallHeight, 0.05]} />
          <meshBasicMaterial color={0x00b3a4} />
        </mesh>
        <mesh position={[width / 2, wallHeight / 2, 0]}>
          <boxGeometry args={[0.05, wallHeight, 0.05]} />
          <meshBasicMaterial color={0x00b3a4} />
        </mesh>
      </group>
      {boundaryState !== 'ready' ? (
        <group userData={{ domeBoundaryState: boundaryState, avatarBarrier: true }}>
          <mesh position={[0, (wallHeight + roofRadius) / 2, -0.02]}>
            <planeGeometry args={[width, wallHeight + roofRadius]} />
            <meshStandardMaterial
              color={boundaryColor(boundaryState)}
              opacity={boundaryState === 'offline' ? 0.48 : 0.78}
              transparent
              wireframe={boundaryState === 'offline'}
              side={THREE.DoubleSide}
            />
          </mesh>
          {(boundaryState === 'draining' || boundaryState === 'blocked') ? [0.25, 0.5, 0.75].map((ratio) => (
            <mesh key={ratio} position={[0, (wallHeight + roofRadius) * ratio, -0.04]}>
              <boxGeometry args={[width, 0.1, 0.08]} />
              <meshBasicMaterial color={boundaryColor(boundaryState)} />
            </mesh>
          )) : null}
        </group>
      ) : null}
    </group>
  );
}

export function FixedDome({
  customization,
  textureUrls,
  openingDirections = DOME_DIRECTIONS,
  connectionDirections = openingDirections,
  boundaryStates = {},
}: {
  customization: DomeCustomizationV1;
  textureUrls: { wall: string | null; floor: string | null };
  openingDirections?: readonly DomeDirection[];
  connectionDirections?: readonly DomeDirection[];
  boundaryStates?: Partial<Record<DomeDirection, DomeBoundaryStateV1>>;
}) {
  const openingKey = openingDirections.join(',');
  const normalizedOpeningDirections = useMemo(
    () => openingKey ? openingKey.split(',') as DomeDirection[] : [],
    [openingKey]
  );
  const innerGeometry = useMemo(
    () => createDomeHemisphereGeometry(DOME_INNER_RADIUS_CM, normalizedOpeningDirections),
    [normalizedOpeningDirections]
  );
  const outerGeometry = useMemo(
    () => createDomeHemisphereGeometry(DOME_OUTER_RADIUS_CM, normalizedOpeningDirections),
    [normalizedOpeningDirections]
  );
  const wallColor = materialColor(customization.surface.wall_material);
  const floorColor = materialColor(customization.surface.floor_material);
  const wallTexture = useDomeTexture(textureUrls.wall);
  const floorTexture = useDomeTexture(textureUrls.floor);
  const floorRadius = DOME_INNER_RADIUS_CM * SCENE_UNITS_PER_CENTIMETER;

  return (
    <group userData={{ fixedDomeSpec: 'fixed_dome_v1', physicsEnabled: true }}>
      <mesh geometry={outerGeometry} castShadow>
        <meshStandardMaterial color={wallTexture ? 0xffffff : wallColor} map={wallTexture} roughness={0.78} side={THREE.FrontSide} />
      </mesh>
      <mesh geometry={innerGeometry} receiveShadow>
        <meshStandardMaterial color={wallTexture ? 0xffffff : wallColor} map={wallTexture} roughness={0.82} side={THREE.BackSide} />
      </mesh>
      <mesh rotation={[-Math.PI / 2, 0, 0]} receiveShadow>
        <circleGeometry args={[floorRadius, 96]} />
        <meshStandardMaterial color={floorTexture ? 0xffffff : floorColor} map={floorTexture} roughness={0.9} side={THREE.DoubleSide} />
      </mesh>
      {connectionDirections.map((direction) => (
        <ConnectionZone
          key={direction}
          direction={direction}
          color={floorColor}
          boundaryState={boundaryStates[direction] ?? 'closed'}
        />
      ))}
    </group>
  );
}
