import { useEffect, useRef, type RefObject } from 'react';
import { useFrame, useThree } from '@react-three/fiber';
import * as THREE from 'three';
import {
  cameraWheelDelta, rotateAvatarCamera, updateAvatarCamera, zoomAvatarCamera,
  type AvatarCameraBounds, type AvatarCameraState,
} from './MetaverseCameraModel';

export function MetaverseFollowCamera({ avatar, bounds, state, enabled }: {
  avatar: RefObject<THREE.Group | null>;
  bounds: RefObject<AvatarCameraBounds>;
  state: RefObject<AvatarCameraState>;
  enabled: boolean;
}) {
  const { camera, gl } = useThree();
  const target = useRef(new THREE.Vector3());
  useEffect(() => {
    const canvas = gl.domElement;
    const ownsInput = () => enabled && document.pointerLockElement === canvas && document.hasFocus() && !document.hidden;
    const mouse = (event: MouseEvent) => {
      if (ownsInput()) rotateAvatarCamera(state.current, event.movementX, event.movementY);
    };
    const wheel = (event: WheelEvent) => {
      if (!ownsInput() || event.ctrlKey || event.metaKey) return;
      event.preventDefault();
      event.stopPropagation();
      zoomAvatarCamera(state.current, cameraWheelDelta(event.deltaY, event.deltaMode, canvas.clientHeight));
    };
    document.addEventListener('mousemove', mouse);
    canvas.addEventListener('wheel', wheel, { passive: false });
    return () => {
      document.removeEventListener('mousemove', mouse);
      canvas.removeEventListener('wheel', wheel);
    };
  }, [enabled, gl, state]);
  useFrame(() => {
    if (avatar.current && camera instanceof THREE.PerspectiveCamera) {
      updateAvatarCamera(camera, avatar.current.position, avatar.current.rotation.y, bounds.current, state.current, target.current);
    }
  });
  return null;
}
