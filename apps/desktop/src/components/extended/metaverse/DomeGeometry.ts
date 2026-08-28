import * as THREE from 'three';

import type { DomeDirection } from '@/lib/api';
import { DOME_DIRECTIONS, openingContains } from './DomeSceneModel';

const SCENE_UNITS_PER_CENTIMETER = 0.01;

function pointFallsInsideOpening(
  x: number,
  y: number,
  z: number,
  openingDirections: ReadonlySet<DomeDirection>
): boolean {
  const xCm = x / SCENE_UNITS_PER_CENTIMETER;
  const yCm = y / SCENE_UNITS_PER_CENTIMETER;
  const zCm = z / SCENE_UNITS_PER_CENTIMETER;
  const northSouth = Math.abs(zCm) >= Math.abs(xCm);
  const direction: DomeDirection = northSouth
    ? (zCm < 0 ? 'north' : 'south')
    : (xCm < 0 ? 'west' : 'east');
  const tangentCm = northSouth ? xCm : zCm;
  return openingDirections.has(direction) && openingContains(tangentCm, yCm);
}

export function createDomeHemisphereGeometry(
  radiusCm: number,
  openingDirections: readonly DomeDirection[] = DOME_DIRECTIONS
): THREE.BufferGeometry {
  const openings = new Set(openingDirections);
  const radius = radiusCm * SCENE_UNITS_PER_CENTIMETER;
  const source = new THREE.SphereGeometry(
    radius,
    96,
    48,
    0,
    Math.PI * 2,
    0,
    Math.PI / 2
  ).toNonIndexed();
  const positions = source.getAttribute('position');
  const kept: number[] = [];
  for (let index = 0; index < positions.count; index += 3) {
    const centroidX = (positions.getX(index) + positions.getX(index + 1) + positions.getX(index + 2)) / 3;
    const centroidY = (positions.getY(index) + positions.getY(index + 1) + positions.getY(index + 2)) / 3;
    const centroidZ = (positions.getZ(index) + positions.getZ(index + 1) + positions.getZ(index + 2)) / 3;
    if (pointFallsInsideOpening(centroidX, centroidY, centroidZ, openings)) continue;
    for (let vertex = 0; vertex < 3; vertex += 1) {
      kept.push(
        positions.getX(index + vertex),
        positions.getY(index + vertex),
        positions.getZ(index + vertex)
      );
    }
  }
  source.dispose();
  const geometry = new THREE.BufferGeometry();
  geometry.setAttribute('position', new THREE.Float32BufferAttribute(kept, 3));
  geometry.computeVertexNormals();
  geometry.computeBoundingSphere();
  return geometry;
}
