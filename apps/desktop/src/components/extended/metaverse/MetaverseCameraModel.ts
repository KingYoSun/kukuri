import * as THREE from 'three';

export type CameraCommand = 'left' | 'right' | 'up' | 'down' | 'in' | 'out' | 'reset';
export type AvatarCameraBounds = { centerY: number; radius: number };
export const FALLBACK_CAMERA_BOUNDS: AvatarCameraBounds = { centerY: 0.8, radius: 1.05 };
export type AvatarCameraState = { yaw: number; pitch: number; zoom: number; reset: boolean };

export function createAvatarCameraState(): AvatarCameraState {
  return { yaw: Math.PI, pitch: 0.28, zoom: 1, reset: true };
}

export function rotateAvatarCamera(state: AvatarCameraState, x: number, y: number) {
  if (!Number.isFinite(x) || !Number.isFinite(y)) return;
  state.yaw -= x * 0.0025;
  state.pitch = THREE.MathUtils.clamp(state.pitch + y * 0.0025, 0.05, 1.2);
}

export function zoomAvatarCamera(state: AvatarCameraState, delta: number) {
  if (!Number.isFinite(delta)) return;
  state.zoom = THREE.MathUtils.clamp(state.zoom * Math.exp(THREE.MathUtils.clamp(delta, -600, 600) * 0.001), 0.45, 3);
}

export function cameraWheelDelta(deltaY: number, deltaMode: number, height: number) {
  return deltaY * (deltaMode === 1 ? 16 : deltaMode === 2 ? height : 1);
}

export function applyCameraCommand(state: AvatarCameraState, command: CameraCommand) {
  if (command === 'reset') state.reset = true;
  else if (command === 'in' || command === 'out') zoomAvatarCamera(state, command === 'in' ? -180 : 180);
  else rotateAvatarCamera(state, command === 'left' ? -100 : command === 'right' ? 100 : 0,
    command === 'up' ? -80 : command === 'down' ? 80 : 0);
}

// A sphere around the visible avatar fits both axes even after orbiting or animating.
export function avatarCameraDistance(bounds: AvatarCameraBounds, fov: number, aspect: number) {
  const vertical = THREE.MathUtils.degToRad(fov) / 2;
  const horizontal = Math.atan(Math.tan(vertical) * Math.max(0.1, aspect));
  return Math.max(3, bounds.radius / Math.sin(Math.min(vertical, horizontal)) * 1.3);
}

export function updateAvatarCamera(
  camera: THREE.PerspectiveCamera,
  position: THREE.Vector3,
  avatarYaw: number,
  bounds: AvatarCameraBounds,
  state: AvatarCameraState,
  target: THREE.Vector3,
) {
  if (state.reset) {
    state.yaw = avatarYaw + Math.PI;
    state.pitch = 0.28;
    state.zoom = 1;
    state.reset = false;
  }
  target.copy(position);
  target.y += bounds.centerY;
  const distance = avatarCameraDistance(bounds, camera.fov, camera.aspect) * state.zoom;
  camera.position.set(
    target.x + Math.sin(state.yaw) * Math.cos(state.pitch) * distance,
    target.y + Math.sin(state.pitch) * distance,
    target.z + Math.cos(state.yaw) * Math.cos(state.pitch) * distance,
  );
  camera.lookAt(target);
}
