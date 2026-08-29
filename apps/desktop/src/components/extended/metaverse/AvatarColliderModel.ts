import { VRMLoaderPlugin, VRMUtils, type VRM } from '@pixiv/three-vrm';
import * as THREE from 'three';
import { GLTFLoader, type GLTF } from 'three/examples/jsm/loaders/GLTFLoader.js';

import type { MetaverseColliderV1 } from '@/lib/api';
import { resolveDomeCollider } from './DomeSceneModel';

export function avatarColliderFromLoadedVrm(
  gltf: Pick<GLTF, 'userData'>,
  vrmRoot: THREE.Object3D
): MetaverseColliderV1 {
  const bounds = new THREE.Box3().setFromObject(vrmRoot);
  const explicitCollider = (gltf.userData.collider ?? vrmRoot.userData.collider) as
    | MetaverseColliderV1
    | undefined;
  return resolveDomeCollider(explicitCollider, {
    min: [
      Math.round(bounds.min.x * 100),
      Math.round(bounds.min.y * 100),
      Math.round(bounds.min.z * 100),
    ],
    max: [
      Math.round(bounds.max.x * 100),
      Math.round(bounds.max.y * 100),
      Math.round(bounds.max.z * 100),
    ],
  });
}

export async function loadAvatarCollider(
  assetUrl: string | null
): Promise<MetaverseColliderV1 | null> {
  if (!assetUrl) return null;
  const loader = new GLTFLoader();
  loader.register((parser) => new VRMLoaderPlugin(parser));
  const gltf = await loader.loadAsync(assetUrl);
  try {
    const vrm = gltf.userData.vrm as VRM | undefined;
    if (!vrm) return null;
    VRMUtils.removeUnnecessaryVertices(gltf.scene);
    VRMUtils.removeUnnecessaryJoints(gltf.scene);
    vrm.scene.scale.setScalar(1);
    VRMUtils.rotateVRM0(vrm);
    vrm.scene.position.y = 0;
    return avatarColliderFromLoadedVrm(gltf, vrm.scene);
  } finally {
    VRMUtils.deepDispose(gltf.scene);
  }
}
