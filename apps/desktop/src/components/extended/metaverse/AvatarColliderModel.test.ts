import * as THREE from 'three';
import { describe, expect, it } from 'vitest';

import { avatarColliderFromLoadedVrm } from './AvatarColliderModel';

describe('AvatarColliderModel', () => {
  it('uses explicit VRM collider metadata before bounding-box capsule fallback', () => {
    const root = new THREE.Group();
    const mesh = new THREE.Mesh(new THREE.BoxGeometry(1, 2, 1));
    mesh.position.y = 1;
    root.add(mesh);

    expect(avatarColliderFromLoadedVrm({ userData: {} }, root)).toEqual({
      shape: 'capsule',
      center: [0, 100, 0],
      radius: 50,
      half_height: 50,
    });

    const explicit = {
      shape: 'cuboid' as const,
      center: [0, 90, 0] as [number, number, number],
      half_extents: [30, 90, 30] as [number, number, number],
    };
    expect(avatarColliderFromLoadedVrm({ userData: { collider: explicit } }, root)).toBe(explicit);

    mesh.geometry.dispose();
  });
});
