import { describe, expect, test } from 'vitest';
import * as THREE from 'three';
import { applyCameraCommand, cameraWheelDelta, createAvatarCameraState, FALLBACK_CAMERA_BOUNDS, rotateAvatarCamera, updateAvatarCamera, zoomAvatarCamera } from './MetaverseCameraModel';

describe('avatar follow camera', () => {
  test.each([0.35, 1, 2.5])('frames the whole avatar at non-origin spawn, aspect %s', (aspect) => {
    const camera = new THREE.PerspectiveCamera(54, aspect, 0.1, 160);
    const state = createAvatarCameraState();
    const position = new THREE.Vector3(8, 0, -12);
    updateAvatarCamera(camera, position, 0.7, FALLBACK_CAMERA_BOUNDS, state, new THREE.Vector3());
    camera.updateMatrixWorld();
    for (const x of [-0.6, 0.6]) for (const y of [0, 1.8]) for (const z of [-0.3, 0.3]) {
      const projected = position.clone().add(new THREE.Vector3(x, y, z)).project(camera);
      expect(Math.abs(projected.x)).toBeLessThan(0.9);
      expect(Math.abs(projected.y)).toBeLessThan(0.9);
      expect(projected.z).toBeGreaterThan(-1);
      expect(projected.z).toBeLessThan(1);
    }
    expect(position.toArray()).toEqual([8, 0, -12]);
  });
  test('follows frame position without changing orbit or avatar rotation and resets around the current avatar', () => {
    const camera = new THREE.PerspectiveCamera(54, 1);
    const state = createAvatarCameraState();
    const target = new THREE.Vector3();
    updateAvatarCamera(camera, new THREE.Vector3(), 0, FALLBACK_CAMERA_BOUNDS, state, target);
    rotateAvatarCamera(state, 120, 70);
    updateAvatarCamera(camera, new THREE.Vector3(), 0, FALLBACK_CAMERA_BOUNDS, state, target);
    const before = camera.position.clone();
    const yaw = state.yaw;
    updateAvatarCamera(camera, new THREE.Vector3(3, 2, -5), 1, FALLBACK_CAMERA_BOUNDS, state, target);
    expect(camera.position.clone().sub(before).toArray()).toEqual([3, 2, -5]);
    expect(state.yaw).toBe(yaw);
    applyCameraCommand(state, 'reset');
    updateAvatarCamera(camera, new THREE.Vector3(3, 2, -5), 1, FALLBACK_CAMERA_BOUNDS, state, target);
    expect(state.yaw).toBe(1 + Math.PI);
    expect(state.zoom).toBe(1);
    expect(target.toArray()).toEqual([3, 2.8, -5]);
  });
  test('bounds pitch and distance and normalizes wheel units', () => {
    const state = createAvatarCameraState();
    rotateAvatarCamera(state, 0, 100000);
    expect(state.pitch).toBe(1.2);
    rotateAvatarCamera(state, 0, -100000);
    expect(state.pitch).toBe(0.05);
    for (let i = 0; i < 20; i++) zoomAvatarCamera(state, 100000);
    expect(state.zoom).toBe(3);
    expect(cameraWheelDelta(2, 1, 500)).toBe(32);
    expect(cameraWheelDelta(2, 2, 500)).toBe(1000);
  });
});
