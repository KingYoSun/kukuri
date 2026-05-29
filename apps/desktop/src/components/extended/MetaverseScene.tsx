import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRMUtils, type VRM } from '@pixiv/three-vrm';
import {
  VRMAnimationLoaderPlugin,
  createVRMAnimationClip,
  type VRMAnimation,
} from '@pixiv/three-vrm-animation';

import type { GameRoomView, SharedRoomObjectV1 } from '@/lib/api';
import {
  DEFAULT_AVATAR_ASSET_URL,
  avatarAnimationForInput,
  type AvatarAnimationState,
  type AvatarAssetStatus,
  type AvatarTransform,
  type MetaverseVec3,
} from './MetaverseSceneModel';

type SceneProps = {
  room: GameRoomView;
  localPeerId: string;
  remoteTransforms: Record<string, AvatarTransform>;
  sharedObject: SharedRoomObjectV1;
  avatarAssetUrl: string | null;
  onLocalTransform: (transform: AvatarTransform) => void;
  onAvatarAssetStatus: (status: AvatarAssetStatus) => void;
};

const AVATAR_ANIMATION_ASSETS = {
  idle: '/animation/Idle_Loop.vrma',
  walk: '/animation/Walk_Loop.vrma',
  sprint: '/animation/Sprint_Loop.vrma',
  jumpStart: '/animation/Jump_Start.vrma',
  jumpLoop: '/animation/Jump_Loop.vrma',
  jumpLand: '/animation/Jump_Land.vrma',
  sittingEnter: '/animation/Sitting_Enter.vrma',
  sittingIdle: '/animation/Sitting_Idle_Loop.vrma',
  sittingExit: '/animation/Sitting_Exit.vrma',
} as const;

type AnimationActionKey = keyof typeof AVATAR_ANIMATION_ASSETS;

type AvatarAnimationRuntime = {
  mixer: THREE.AnimationMixer;
  actions: Partial<Record<AnimationActionKey, THREE.AnimationAction>>;
  activeAction: THREE.AnimationAction | null;
  activeKey: AnimationActionKey | null;
  jumpActive: boolean;
  sittingActive: boolean;
  jumpLoopTimeoutId: number | null;
  sittingExitTimeoutId: number | null;
};

function toSceneUnit(value: number) {
  return value / 100;
}

function scenePosition(values: MetaverseVec3) {
  return new THREE.Vector3(toSceneUnit(values[0]), toSceneUnit(values[1]), toSceneUnit(values[2]));
}

function makeAvatarMesh(color: number) {
  const group = new THREE.Group();
  const body = new THREE.Mesh(
    new THREE.CapsuleGeometry(0.28, 0.72, 6, 12),
    new THREE.MeshStandardMaterial({ color, roughness: 0.7 })
  );
  body.position.y = 0.72;
  const head = new THREE.Mesh(
    new THREE.SphereGeometry(0.22, 16, 16),
    new THREE.MeshStandardMaterial({ color: 0xf5d0c5, roughness: 0.65 })
  );
  head.position.y = 1.34;
  group.add(body, head);
  return group;
}

function disposeObjectTree(object: THREE.Object3D) {
  object.traverse((child) => {
    if (child instanceof THREE.Mesh) {
      child.geometry.dispose();
      const materials = Array.isArray(child.material) ? child.material : [child.material];
      for (const material of materials) {
        material.dispose();
      }
    }
  });
}

function initialAvatarTransform(
  roomId: string,
  localPeerId: string,
  spawnPosition?: MetaverseVec3,
  spawnRotation?: MetaverseVec3
): AvatarTransform {
  return {
    roomId,
    peerId: localPeerId,
    seq: 0,
    position: spawnPosition ?? [0, 0, 260],
    rotation: spawnRotation ?? [0, 180, 0],
    animation: 'idle',
    sentAt: 0,
  };
}

function isEditableTarget(target: EventTarget | null) {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName.toLowerCase();
  return tagName === 'input' || tagName === 'textarea' || tagName === 'select' || target.isContentEditable;
}

function movementVectorFromKeys(keys: Set<string>) {
  const x = (keys.has('d') || keys.has('arrowright') ? 1 : 0) - (keys.has('a') || keys.has('arrowleft') ? 1 : 0);
  const z = (keys.has('s') || keys.has('arrowdown') ? 1 : 0) - (keys.has('w') || keys.has('arrowup') ? 1 : 0);
  return { x, z, moving: x !== 0 || z !== 0 };
}

function avatarTransformsEqual(left: AvatarTransform | null, right: AvatarTransform) {
  return (
    left !== null &&
    left.position[0] === right.position[0] &&
    left.position[1] === right.position[1] &&
    left.position[2] === right.position[2] &&
    left.rotation[0] === right.rotation[0] &&
    left.rotation[1] === right.rotation[1] &&
    left.rotation[2] === right.rotation[2] &&
    left.animation === right.animation
  );
}

function loadVRMAnimation(loader: GLTFLoader, url: string): Promise<VRMAnimation | null> {
  return new Promise((resolve) => {
    loader.load(
      url,
      (gltf) => {
        const animations = gltf.userData.vrmAnimations as VRMAnimation[] | undefined;
        resolve(animations?.[0] ?? null);
      },
      undefined,
      () => resolve(null)
    );
  });
}

function fadeToAnimation(runtime: AvatarAnimationRuntime, key: AnimationActionKey, fadeSeconds = 0.16) {
  const nextAction = runtime.actions[key];
  if (!nextAction || runtime.activeKey === key) {
    return;
  }
  nextAction.reset();
  nextAction.enabled = true;
  nextAction.setEffectiveTimeScale(1);
  nextAction.setEffectiveWeight(1);
  if (runtime.activeAction) {
    runtime.activeAction.crossFadeTo(nextAction, fadeSeconds, false);
  }
  nextAction.play();
  runtime.activeAction = nextAction;
  runtime.activeKey = key;
}

function loopKeyForAnimation(animation: AvatarAnimationState): AnimationActionKey {
  if (animation === 'walk') {
    return 'walk';
  }
  if (animation === 'sprint') {
    return 'sprint';
  }
  if (animation === 'sitting') {
    return 'sittingIdle';
  }
  return 'idle';
}

function playLoopAnimation(runtime: AvatarAnimationRuntime, animation: AvatarAnimationState) {
  fadeToAnimation(runtime, loopKeyForAnimation(animation));
}

function playJumpAnimation(runtime: AvatarAnimationRuntime) {
  if (runtime.jumpActive || runtime.sittingActive) {
    return;
  }
  runtime.jumpActive = true;
  fadeToAnimation(runtime, 'jumpStart', 0.08);
}

function playSittingEnter(runtime: AvatarAnimationRuntime) {
  if (runtime.jumpActive || runtime.sittingActive) {
    return;
  }
  runtime.sittingActive = true;
  fadeToAnimation(runtime, 'sittingEnter', 0.12);
}

function playSittingExit(runtime: AvatarAnimationRuntime, nextAnimation: AvatarAnimationState) {
  if (runtime.jumpActive || !runtime.sittingActive) {
    return;
  }
  runtime.sittingActive = false;
  fadeToAnimation(runtime, 'sittingExit', 0.12);
  if (runtime.sittingExitTimeoutId) {
    window.clearTimeout(runtime.sittingExitTimeoutId);
  }
  runtime.sittingExitTimeoutId = window.setTimeout(() => {
    runtime.sittingExitTimeoutId = null;
    if (!runtime.sittingActive && !runtime.jumpActive) {
      playLoopAnimation(runtime, nextAnimation);
    }
  }, 300);
}

export function MetaverseScene({
  room,
  localPeerId,
  remoteTransforms,
  sharedObject,
  avatarAssetUrl,
  onLocalTransform,
  onAvatarAssetStatus,
}: SceneProps) {
  const mountRef = useRef<HTMLDivElement | null>(null);
  const spawnPosition = room.metaverse?.default_spawn.position;
  const spawnRotation = room.metaverse?.default_spawn.rotation;
  const localTransformRef = useRef<AvatarTransform>(
    initialAvatarTransform(room.room_id, localPeerId, spawnPosition, spawnRotation)
  );
  const seqRef = useRef(0);
  const lastSentAtRef = useRef(0);
  const lastPublishedTransformRef = useRef<AvatarTransform | null>(null);
  const keysRef = useRef(new Set<string>());
  const remoteGroupsRef = useRef(new Map<string, THREE.Group>());
  const localAvatarRef = useRef<THREE.Group | null>(null);
  const sharedObjectRef = useRef<THREE.Mesh | null>(null);
  const onLocalTransformRef = useRef(onLocalTransform);
  const onAvatarAssetStatusRef = useRef(onAvatarAssetStatus);
  const remoteTransformsRef = useRef(remoteTransforms);
  const sharedObjectStateRef = useRef(sharedObject);

  useEffect(() => {
    onLocalTransformRef.current = onLocalTransform;
  }, [onLocalTransform]);

  useEffect(() => {
    onAvatarAssetStatusRef.current = onAvatarAssetStatus;
  }, [onAvatarAssetStatus]);

  useEffect(() => {
    remoteTransformsRef.current = remoteTransforms;
  }, [remoteTransforms]);

  useEffect(() => {
    const nextTransform = initialAvatarTransform(room.room_id, localPeerId, spawnPosition, spawnRotation);
    localTransformRef.current = nextTransform;
    seqRef.current = 0;
    lastSentAtRef.current = 0;
    lastPublishedTransformRef.current = null;
    keysRef.current.clear();
    if (localAvatarRef.current) {
      localAvatarRef.current.position.copy(scenePosition(nextTransform.position));
      localAvatarRef.current.rotation.y = THREE.MathUtils.degToRad(nextTransform.rotation[1]);
    }
  }, [localPeerId, room.room_id, spawnPosition, spawnRotation]);

  useEffect(() => {
    sharedObjectStateRef.current = sharedObject;
    if (sharedObjectRef.current) {
      sharedObjectRef.current.position.copy(scenePosition(sharedObject.position));
      sharedObjectRef.current.rotation.set(
        THREE.MathUtils.degToRad(sharedObject.rotation[0]),
        THREE.MathUtils.degToRad(sharedObject.rotation[1]),
        THREE.MathUtils.degToRad(sharedObject.rotation[2])
      );
      sharedObjectRef.current.scale.set(
        Math.max(0.1, toSceneUnit(sharedObject.scale[0])),
        Math.max(0.1, toSceneUnit(sharedObject.scale[1])),
        Math.max(0.1, toSceneUnit(sharedObject.scale[2]))
      );
    }
  }, [sharedObject]);

  useEffect(() => {
    const mount = mountRef.current;
    if (!mount) {
      return;
    }
    const avatarAssetRuntime: { vrm: VRM | null; loadedRoot: THREE.Object3D | null } = {
      vrm: null,
      loadedRoot: null,
    };
    const animationRuntimeRef: { current: AvatarAnimationRuntime | null } = {
      current: null,
    };
    let disposed = false;

    const scene = new THREE.Scene();
    scene.background = new THREE.Color(0x101318);
    const camera = new THREE.PerspectiveCamera(58, 1, 0.1, 100);
    camera.position.set(0, 4.2, 6.5);
    camera.lookAt(0, 0.8, 0);

    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setSize(mount.clientWidth, mount.clientHeight);
    mount.appendChild(renderer.domElement);

    const hemi = new THREE.HemisphereLight(0xb7d7ff, 0x29351f, 2.2);
    const key = new THREE.DirectionalLight(0xffffff, 2.4);
    key.position.set(4, 8, 3);
    scene.add(hemi, key);

    const grid = new THREE.GridHelper(12, 12, 0x4f9f78, 0x30423a);
    scene.add(grid);

    const ground = new THREE.Mesh(
      new THREE.PlaneGeometry(12, 12),
      new THREE.MeshStandardMaterial({ color: 0x1c2a25, roughness: 0.9 })
    );
    ground.rotation.x = -Math.PI / 2;
    ground.position.y = -0.01;
    scene.add(ground);

    const localAvatar = makeAvatarMesh(0x4f9fef);
    localAvatar.position.copy(scenePosition(localTransformRef.current.position));
    localAvatarRef.current = localAvatar;
    scene.add(localAvatar);
    onAvatarAssetStatusRef.current('loading');

    const avatarLoader = new GLTFLoader();
    avatarLoader.register((parser) => new VRMLoaderPlugin(parser));
    const avatarUrl = avatarAssetUrl ?? DEFAULT_AVATAR_ASSET_URL;
    avatarLoader.load(
      avatarUrl,
      (gltf) => {
        const vrm = gltf.userData.vrm as VRM | undefined;
        if (disposed || !vrm) {
          onAvatarAssetStatusRef.current('fallback-primitive');
          return;
        }
        VRMUtils.removeUnnecessaryVertices(gltf.scene);
        VRMUtils.removeUnnecessaryJoints(gltf.scene);
        const vrmRoot = vrm.scene;
        vrmRoot.scale.setScalar(0.9);
        vrmRoot.rotation.y = Math.PI;
        vrmRoot.position.y = 0;
        while (localAvatar.children.length > 0) {
          const child = localAvatar.children[0];
          localAvatar.remove(child);
          disposeObjectTree(child);
        }
        localAvatar.add(vrmRoot);
        avatarAssetRuntime.vrm = vrm;
        avatarAssetRuntime.loadedRoot = vrmRoot;
        onAvatarAssetStatusRef.current(avatarAssetUrl ? 'blob-vrm' : 'sample-vrm');
        const animationLoader = new GLTFLoader();
        animationLoader.register((parser) => new VRMAnimationLoaderPlugin(parser));
        const mixer = new THREE.AnimationMixer(vrm.scene);
        const runtime: AvatarAnimationRuntime = {
          mixer,
          actions: {},
          activeAction: null,
          activeKey: null,
          jumpActive: false,
          sittingActive: false,
          jumpLoopTimeoutId: null,
          sittingExitTimeoutId: null,
        };
        mixer.addEventListener('finished', (event) => {
          if (event.action === runtime.actions.jumpStart) {
            fadeToAnimation(runtime, 'jumpLoop', 0.06);
            if (runtime.jumpLoopTimeoutId) {
              window.clearTimeout(runtime.jumpLoopTimeoutId);
            }
            runtime.jumpLoopTimeoutId = window.setTimeout(() => {
              runtime.jumpLoopTimeoutId = null;
              fadeToAnimation(runtime, 'jumpLand', 0.06);
            }, 180);
            return;
          }
          if (event.action === runtime.actions.jumpLand) {
            runtime.jumpActive = false;
            playLoopAnimation(runtime, localTransformRef.current.animation);
            return;
          }
          if (event.action === runtime.actions.sittingEnter) {
            playLoopAnimation(runtime, 'sitting');
            return;
          }
          if (event.action === runtime.actions.sittingExit) {
            playLoopAnimation(runtime, localTransformRef.current.animation);
          }
        });
        animationRuntimeRef.current = runtime;
        void Promise.all(
          Object.entries(AVATAR_ANIMATION_ASSETS).map(async ([key, url]) => {
            const vrmAnimation = await loadVRMAnimation(animationLoader, url);
            if (!vrmAnimation || disposed) {
              return;
            }
            const action = mixer.clipAction(createVRMAnimationClip(vrmAnimation, vrm));
            if (key === 'jumpStart' || key === 'jumpLand' || key === 'sittingEnter' || key === 'sittingExit') {
              action.setLoop(THREE.LoopOnce, 1);
              action.clampWhenFinished = true;
            } else {
              action.setLoop(THREE.LoopRepeat, Number.POSITIVE_INFINITY);
            }
            runtime.actions[key as AnimationActionKey] = action;
          })
        ).then(() => {
          if (!disposed) {
            playLoopAnimation(runtime, localTransformRef.current.animation);
          }
        });
      },
      undefined,
      () => {
        if (!disposed) {
          onAvatarAssetStatusRef.current('fallback-primitive');
        }
      }
    );

    const objectMesh = new THREE.Mesh(
      new THREE.BoxGeometry(1, 1, 1),
      new THREE.MeshStandardMaterial({ color: 0xf3b35d, roughness: 0.55 })
    );
    objectMesh.position.copy(scenePosition(sharedObjectStateRef.current.position));
    objectMesh.scale.set(1, 1, 1);
    sharedObjectRef.current = objectMesh;
    scene.add(objectMesh);

    let sittingRequested = false;
    let jumpRequested = false;
    let primitiveJumpUntil = 0;

    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) {
        return;
      }
      const key = event.key.toLowerCase();
      if (event.key === 'Shift') {
        keysRef.current.add('shift');
        return;
      }
      if (event.code === 'Space') {
        event.preventDefault();
        if (!event.repeat && !sittingRequested) {
          jumpRequested = true;
        }
        return;
      }
      if (key === 'c') {
        if (!event.repeat) {
          sittingRequested = !sittingRequested;
        }
        return;
      }
      if (
        key === 'w' ||
        key === 'a' ||
        key === 's' ||
        key === 'd' ||
        event.key.startsWith('Arrow')
      ) {
        keysRef.current.add(key);
      }
    };
    const handleKeyUp = (event: KeyboardEvent) => {
      keysRef.current.delete(event.key.toLowerCase());
      if (event.key === 'Shift') {
        keysRef.current.delete('shift');
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);

    const resizeObserver = new ResizeObserver(([entry]) => {
      const width = Math.max(1, entry.contentRect.width);
      const height = Math.max(1, entry.contentRect.height);
      camera.aspect = width / height;
      camera.updateProjectionMatrix();
      renderer.setSize(width, height);
    });
    resizeObserver.observe(mount);

    let frameId = 0;
    let lastFrameAt = performance.now();
    const tick = (frameAt: number) => {
      const deltaSeconds = Math.min(0.05, (frameAt - lastFrameAt) / 1000);
      lastFrameAt = frameAt;
      const keys = keysRef.current;
      const { x, z, moving } = movementVectorFromKeys(keys);
      const runtime = animationRuntimeRef.current;
      if (jumpRequested) {
        if (runtime) {
          playJumpAnimation(runtime);
        }
        primitiveJumpUntil = frameAt + 650;
        jumpRequested = false;
      }
      const baseAnimation = avatarAnimationForInput(keys, sittingRequested);
      const currentRuntimeState = runtime?.jumpActive || frameAt < primitiveJumpUntil ? 'jump' : baseAnimation;
      if (runtime && baseAnimation === 'sitting' && !runtime.sittingActive) {
        playSittingEnter(runtime);
      } else if (runtime && baseAnimation !== 'sitting' && runtime.sittingActive) {
        playSittingExit(runtime, baseAnimation);
      } else if (runtime && !runtime.jumpActive && !runtime.sittingActive && runtime.activeKey !== 'sittingExit') {
        playLoopAnimation(runtime, baseAnimation);
      }

      if (moving && baseAnimation !== 'sitting') {
        const length = Math.hypot(x, z) || 1;
        const speed = baseAnimation === 'sprint' ? 380 : 220;
        const current = localTransformRef.current;
        const nextPosition: MetaverseVec3 = [
          current.position[0] + Math.round((x / length) * speed * deltaSeconds),
          current.position[1],
          current.position[2] + Math.round((z / length) * speed * deltaSeconds),
        ];
        const yaw = Math.round(THREE.MathUtils.radToDeg(Math.atan2(x, z)));
        localTransformRef.current = {
          ...current,
          seq: seqRef.current,
          position: nextPosition,
          rotation: [0, yaw, 0],
          animation: currentRuntimeState,
          sentAt: Date.now(),
        };
        localAvatar.position.copy(scenePosition(nextPosition));
        localAvatar.rotation.y = THREE.MathUtils.degToRad(yaw);
      } else if (localTransformRef.current.animation !== currentRuntimeState) {
        localTransformRef.current = {
          ...localTransformRef.current,
          seq: seqRef.current,
          animation: currentRuntimeState,
          sentAt: Date.now(),
        };
      }
      runtime?.mixer.update(deltaSeconds);
      avatarAssetRuntime.vrm?.update(deltaSeconds);

      if (frameAt - lastSentAtRef.current >= 100) {
        lastSentAtRef.current = frameAt;
        const nextTransform = {
          ...localTransformRef.current,
          seq: seqRef.current + 1,
          sentAt: Date.now(),
        };
        if (!avatarTransformsEqual(lastPublishedTransformRef.current, nextTransform)) {
          seqRef.current += 1;
          lastPublishedTransformRef.current = nextTransform;
          onLocalTransformRef.current(nextTransform);
        }
      }

      const seenPeers = new Set(Object.keys(remoteTransformsRef.current));
      for (const [peerId, transform] of Object.entries(remoteTransformsRef.current)) {
        let group = remoteGroupsRef.current.get(peerId);
        if (!group) {
          group = makeAvatarMesh(0xe37070);
          remoteGroupsRef.current.set(peerId, group);
          scene.add(group);
        }
        group.position.copy(scenePosition(transform.position));
        group.rotation.y = THREE.MathUtils.degToRad(transform.rotation[1]);
        group.visible = Date.now() - transform.sentAt < 15_000;
      }
      for (const [peerId, group] of remoteGroupsRef.current) {
        if (!seenPeers.has(peerId)) {
          scene.remove(group);
          remoteGroupsRef.current.delete(peerId);
        }
      }

      objectMesh.rotation.y += 0.25 * deltaSeconds;
      renderer.render(scene, camera);
      frameId = window.requestAnimationFrame(tick);
    };
    frameId = window.requestAnimationFrame(tick);

    return () => {
      window.cancelAnimationFrame(frameId);
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
      resizeObserver.disconnect();
      renderer.dispose();
      mount.removeChild(renderer.domElement);
      localAvatarRef.current = null;
      sharedObjectRef.current = null;
      if (animationRuntimeRef.current?.jumpLoopTimeoutId) {
        window.clearTimeout(animationRuntimeRef.current.jumpLoopTimeoutId);
      }
      if (animationRuntimeRef.current?.sittingExitTimeoutId) {
        window.clearTimeout(animationRuntimeRef.current.sittingExitTimeoutId);
      }
      animationRuntimeRef.current?.mixer.stopAllAction();
      if (avatarAssetRuntime.vrm) {
        animationRuntimeRef.current?.mixer.uncacheRoot(avatarAssetRuntime.vrm.scene);
      }
      animationRuntimeRef.current = null;
      if (avatarAssetRuntime.loadedRoot) {
        disposeObjectTree(avatarAssetRuntime.loadedRoot);
      }
      avatarAssetRuntime.vrm = null;
      avatarAssetRuntime.loadedRoot = null;
      disposed = true;
    };
  }, [avatarAssetUrl, localPeerId, room.room_id, room.metaverse?.default_spawn.position, room.metaverse?.default_spawn.rotation]);

  return <div className='metaverse-viewport' ref={mountRef} aria-label='Metaverse room viewport' />;
}

