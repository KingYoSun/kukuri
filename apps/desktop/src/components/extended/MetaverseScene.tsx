import { useEffect, useMemo, useRef, useState, type ReactNode, type RefObject } from 'react';
import { Canvas, useFrame, useThree } from '@react-three/fiber';
import { Html } from '@react-three/drei';
import { MonitorPause } from 'lucide-react';
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
  AVATAR_GROUND_Y,
  DEFAULT_AVATAR_ASSET_URL,
  METAVERSE_AVATAR_IDLE_SEND_INTERVAL_MS,
  METAVERSE_AVATAR_MOVING_SEND_INTERVAL_MS,
  METAVERSE_REMOTE_AVATAR_SMOOTHING_SECONDS,
  METAVERSE_ROOM_STALE_MS,
  avatarAnimationForInput,
  initialAvatarTransform,
  isNewerRemoteTransform,
  stepAvatarJump,
  type AvatarAnimationState,
  type AvatarAssetStatus,
  type AvatarPhysicsState,
  type AvatarTransform,
  type LatestChatBubble,
  type MetaverseRoomConnectionState,
  type MetaverseVec3,
  type PeerPresence,
} from './MetaverseSceneModel';

type SceneProps = {
  room: GameRoomView;
  localPeerId: string;
  remoteTransforms: Record<string, AvatarTransform>;
  peerPresence: Record<string, PeerPresence>;
  sharedObject: SharedRoomObjectV1;
  avatarAssetUrl: string | null;
  latestChatByPeer: Record<string, LatestChatBubble>;
  connectionState: MetaverseRoomConnectionState;
  now: number;
  hud: ReactNode;
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

function makePrimitiveAvatar(color: number) {
  return (
    <>
      <mesh position={[0, 0.72, 0]}>
        <capsuleGeometry args={[0.28, 0.72, 6, 12]} />
        <meshStandardMaterial color={color} roughness={0.7} />
      </mesh>
      <mesh position={[0, 1.34, 0]}>
        <sphereGeometry args={[0.22, 16, 16]} />
        <meshStandardMaterial color={0xf5d0c5} roughness={0.65} />
      </mesh>
    </>
  );
}

function AvatarChatBubble({ bubble }: { bubble?: LatestChatBubble }) {
  if (!bubble) {
    return null;
  }
  return (
    <Html position={[0, 1.9, 0]} center distanceFactor={8} occlude={false}>
      <div className='metaverse-avatar-bubble'>
        <span>{bubble.body}</span>
      </div>
    </Html>
  );
}

function AvatarStaleIndicator({ stale }: { stale: boolean }) {
  if (!stale) {
    return null;
  }
  return (
    <Html position={[0.32, 1.9, 0]} center distanceFactor={8} occlude={false}>
      <div className='metaverse-avatar-stale-icon' aria-label='Remote avatar stale'>
        <MonitorPause className='size-3' aria-hidden='true' />
      </div>
    </Html>
  );
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

function AvatarModel({
  assetUrl,
  color,
  animationRef,
  statusTarget,
}: {
  assetUrl: string;
  color: number;
  animationRef?: RefObject<AvatarAnimationState>;
  statusTarget?: (status: AvatarAssetStatus) => void;
}) {
  const groupRef = useRef<THREE.Group | null>(null);
  const vrmRuntimeRef = useRef<{ vrm: VRM | null; loadedRoot: THREE.Object3D | null }>({
    vrm: null,
    loadedRoot: null,
  });
  const animationRuntimeRef = useRef<AvatarAnimationRuntime | null>(null);
  const [visiblePrimitive, setVisiblePrimitive] = useState(true);

  useEffect(() => {
    const group = groupRef.current;
    if (!group) {
      return;
    }
    let disposed = false;
    setVisiblePrimitive(true);
    statusTarget?.('loading');
    const avatarLoader = new GLTFLoader();
    avatarLoader.register((parser) => new VRMLoaderPlugin(parser));
    avatarLoader.load(
      assetUrl,
      (gltf) => {
        const vrm = gltf.userData.vrm as VRM | undefined;
        if (disposed || !vrm) {
          statusTarget?.('fallback-primitive');
          return;
        }
        VRMUtils.removeUnnecessaryVertices(gltf.scene);
        VRMUtils.removeUnnecessaryJoints(gltf.scene);
        const vrmRoot = vrm.scene;
        vrmRoot.scale.setScalar(0.9);
        vrmRoot.rotation.y = Math.PI;
        vrmRoot.position.y = 0;
        group.add(vrmRoot);
        setVisiblePrimitive(false);
        vrmRuntimeRef.current = { vrm, loadedRoot: vrmRoot };
        statusTarget?.(assetUrl === DEFAULT_AVATAR_ASSET_URL ? 'sample-vrm' : 'blob-vrm');

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
            playLoopAnimation(runtime, 'idle');
            return;
          }
          if (event.action === runtime.actions.sittingEnter) {
            playLoopAnimation(runtime, 'sitting');
            return;
          }
          if (event.action === runtime.actions.sittingExit) {
            playLoopAnimation(runtime, 'idle');
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
            playLoopAnimation(runtime, 'idle');
          }
        });
      },
      undefined,
      () => {
        if (!disposed) {
          statusTarget?.('fallback-primitive');
        }
      }
    );

    return () => {
      disposed = true;
      const runtime = animationRuntimeRef.current;
      if (runtime?.jumpLoopTimeoutId) {
        window.clearTimeout(runtime.jumpLoopTimeoutId);
      }
      if (runtime?.sittingExitTimeoutId) {
        window.clearTimeout(runtime.sittingExitTimeoutId);
      }
      runtime?.mixer.stopAllAction();
      if (vrmRuntimeRef.current.vrm) {
        runtime?.mixer.uncacheRoot(vrmRuntimeRef.current.vrm.scene);
      }
      if (vrmRuntimeRef.current.loadedRoot) {
        group.remove(vrmRuntimeRef.current.loadedRoot);
        disposeObjectTree(vrmRuntimeRef.current.loadedRoot);
      }
      animationRuntimeRef.current = null;
      vrmRuntimeRef.current = { vrm: null, loadedRoot: null };
    };
  }, [assetUrl, statusTarget]);

  useFrame((_, delta) => {
    const runtime = animationRuntimeRef.current;
    const animation = animationRef?.current ?? 'idle';
    if (runtime) {
      if (animation === 'jump') {
        playJumpAnimation(runtime);
      } else if (animation === 'sitting' && !runtime.sittingActive) {
        playSittingEnter(runtime);
      } else if (animation !== 'sitting' && runtime.sittingActive) {
        playSittingExit(runtime, animation);
      } else if (!runtime.jumpActive && !runtime.sittingActive && runtime.activeKey !== 'sittingExit') {
        playLoopAnimation(runtime, animation);
      }
    }
    runtime?.mixer.update(delta);
    vrmRuntimeRef.current.vrm?.update(delta);
  });

  return (
    <group ref={groupRef}>
      {visiblePrimitive ? makePrimitiveAvatar(color) : null}
    </group>
  );
}

function LocalAvatar({
  room,
  localPeerId,
  avatarAssetUrl,
  chatBubble,
  onLocalTransform,
  onAvatarAssetStatus,
}: Pick<SceneProps, 'room' | 'localPeerId' | 'avatarAssetUrl' | 'onLocalTransform' | 'onAvatarAssetStatus'> & {
  chatBubble?: LatestChatBubble;
}) {
  const groupRef = useRef<THREE.Group | null>(null);
  const spawnPosition = room.metaverse?.default_spawn.position;
  const spawnRotation = room.metaverse?.default_spawn.rotation;
  const spawnPositionKey = spawnPosition?.join(',');
  const spawnRotationKey = spawnRotation?.join(',');
  const transformRef = useRef(initialAvatarTransform(room.room_id, localPeerId, spawnPosition, spawnRotation));
  const physicsRef = useRef<AvatarPhysicsState>({ verticalVelocity: 0, grounded: true });
  const seqRef = useRef(0);
  const lastSentAtRef = useRef(0);
  const lastPublishedTransformRef = useRef<AvatarTransform | null>(null);
  const keysRef = useRef(new Set<string>());
  const animationRef = useRef<AvatarAnimationState>('idle');
  const jumpRequestedRef = useRef(false);
  const sittingRequestedRef = useRef(false);
  const onLocalTransformRef = useRef(onLocalTransform);

  useEffect(() => {
    onLocalTransformRef.current = onLocalTransform;
  }, [onLocalTransform]);

  useEffect(() => {
    const nextSpawnPosition = spawnPositionKey
      ? (spawnPositionKey.split(',').map(Number) as MetaverseVec3)
      : undefined;
    const nextSpawnRotation = spawnRotationKey
      ? (spawnRotationKey.split(',').map(Number) as MetaverseVec3)
      : undefined;
    const nextTransform = initialAvatarTransform(room.room_id, localPeerId, nextSpawnPosition, nextSpawnRotation);
    transformRef.current = nextTransform;
    physicsRef.current = { verticalVelocity: 0, grounded: nextTransform.position[1] <= AVATAR_GROUND_Y };
    seqRef.current = 0;
    lastSentAtRef.current = 0;
    lastPublishedTransformRef.current = null;
    keysRef.current.clear();
    if (groupRef.current) {
      groupRef.current.position.copy(scenePosition(nextTransform.position));
      groupRef.current.rotation.y = THREE.MathUtils.degToRad(nextTransform.rotation[1]);
    }
  }, [localPeerId, room.room_id, spawnPositionKey, spawnRotationKey]);

  useEffect(() => {
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
        if (!event.repeat && !sittingRequestedRef.current) {
          jumpRequestedRef.current = true;
        }
        return;
      }
      if (key === 'c') {
        if (!event.repeat) {
          sittingRequestedRef.current = !sittingRequestedRef.current;
        }
        return;
      }
      if (key === 'w' || key === 'a' || key === 's' || key === 'd' || event.key.startsWith('Arrow')) {
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
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, []);

  useFrame((_, deltaSeconds) => {
    const delta = Math.min(0.05, deltaSeconds);
    const keys = keysRef.current;
    const { x, z, moving } = movementVectorFromKeys(keys);
    const baseAnimation = avatarAnimationForInput(keys, sittingRequestedRef.current);
    const current = transformRef.current;
    const jumpStep = stepAvatarJump(current.position, physicsRef.current, delta, jumpRequestedRef.current);
    jumpRequestedRef.current = false;
    physicsRef.current = jumpStep.physics;

    let nextPosition = jumpStep.position;
    let nextRotation = current.rotation;
    if (moving && baseAnimation !== 'sitting') {
      const length = Math.hypot(x, z) || 1;
      const speed = baseAnimation === 'sprint' ? 380 : 220;
      nextPosition = [
        nextPosition[0] + Math.round((x / length) * speed * delta),
        nextPosition[1],
        nextPosition[2] + Math.round((z / length) * speed * delta),
      ];
      nextRotation = [0, Math.round(THREE.MathUtils.radToDeg(Math.atan2(x, z))), 0];
    }

    const animation = !physicsRef.current.grounded ? 'jump' : baseAnimation;
    animationRef.current = animation;
    transformRef.current = {
      ...current,
      position: nextPosition,
      rotation: nextRotation,
      animation,
      sentAt: Date.now(),
    };

    if (groupRef.current) {
      groupRef.current.position.copy(scenePosition(nextPosition));
      groupRef.current.rotation.y = THREE.MathUtils.degToRad(nextRotation[1]);
    }

    const sendInterval =
      moving || animation === 'jump'
        ? METAVERSE_AVATAR_MOVING_SEND_INTERVAL_MS
        : METAVERSE_AVATAR_IDLE_SEND_INTERVAL_MS;
    const frameAt = performance.now();
    if (frameAt - lastSentAtRef.current >= sendInterval) {
      lastSentAtRef.current = frameAt;
      const nextTransform = {
        ...transformRef.current,
        seq: seqRef.current + 1,
        sentAt: Date.now(),
      };
      if (!avatarTransformsEqual(lastPublishedTransformRef.current, nextTransform)) {
        seqRef.current += 1;
        lastPublishedTransformRef.current = nextTransform;
        transformRef.current = nextTransform;
        onLocalTransformRef.current(nextTransform);
      }
    }
  });

  return (
    <group ref={groupRef}>
      <AvatarModel
        assetUrl={avatarAssetUrl ?? DEFAULT_AVATAR_ASSET_URL}
        color={0x4f9fef}
        animationRef={animationRef}
        statusTarget={onAvatarAssetStatus}
      />
      <AvatarChatBubble bubble={chatBubble} />
    </group>
  );
}

function RemoteAvatar({
  transform,
  presence,
  chatBubble,
  connectionState,
  now,
}: {
  transform: AvatarTransform;
  presence: PeerPresence | null;
  chatBubble?: LatestChatBubble;
  connectionState: MetaverseRoomConnectionState;
  now: number;
}) {
  const groupRef = useRef<THREE.Group | null>(null);
  const targetRef = useRef(transform);
  const animationRef = useRef<AvatarAnimationState>(transform.animation);
  const initializedRef = useRef(false);

  useEffect(() => {
    if (isNewerRemoteTransform(targetRef.current, transform)) {
      targetRef.current = transform;
      animationRef.current = transform.animation;
    }
  }, [transform]);

  useFrame((_, deltaSeconds) => {
    const group = groupRef.current;
    if (!group) {
      return;
    }
    const target = targetRef.current;
    const targetPosition = scenePosition(target.position);
    if (!initializedRef.current) {
      group.position.copy(targetPosition);
      group.rotation.y = THREE.MathUtils.degToRad(target.rotation[1]);
      initializedRef.current = true;
      return;
    }
    const alpha = 1 - Math.exp(-deltaSeconds / METAVERSE_REMOTE_AVATAR_SMOOTHING_SECONDS);
    group.position.lerp(targetPosition, Math.min(1, alpha));
    const targetYaw = THREE.MathUtils.degToRad(target.rotation[1]);
    group.rotation.y = THREE.MathUtils.lerp(group.rotation.y, targetYaw, Math.min(1, alpha));
  });

  const stale = connectionState !== 'live' || now - transform.sentAt > METAVERSE_ROOM_STALE_MS;

  return (
    <group
      ref={groupRef}
      userData={{ displayName: presence?.displayName ?? transform.peerId }}
    >
      <AvatarModel
        assetUrl={presence?.avatarAssetUrl || DEFAULT_AVATAR_ASSET_URL}
        color={0xe37070}
        animationRef={animationRef}
      />
      <AvatarChatBubble bubble={chatBubble} />
      <AvatarStaleIndicator stale={stale} />
    </group>
  );
}

function SharedObject({ object }: { object: SharedRoomObjectV1 }) {
  const position = scenePosition(object.position);
  const rotation: [number, number, number] = [
    THREE.MathUtils.degToRad(object.rotation[0]),
    THREE.MathUtils.degToRad(object.rotation[1]),
    THREE.MathUtils.degToRad(object.rotation[2]),
  ];
  const scale: [number, number, number] = [
    Math.max(0.1, toSceneUnit(object.scale[0])),
    Math.max(0.1, toSceneUnit(object.scale[1])),
    Math.max(0.1, toSceneUnit(object.scale[2])),
  ];

  return (
    <mesh position={position} rotation={rotation} scale={scale}>
      <boxGeometry args={[1, 1, 1]} />
      <meshStandardMaterial color={0xf3b35d} roughness={0.55} />
    </mesh>
  );
}

function SceneContents({
  room,
  localPeerId,
  remoteTransforms,
  peerPresence,
  sharedObject,
  avatarAssetUrl,
  latestChatByPeer,
  connectionState,
  now,
  onLocalTransform,
  onAvatarAssetStatus,
}: Omit<SceneProps, 'hud'>) {
  const remoteEntries = useMemo(() => Object.entries(remoteTransforms), [remoteTransforms]);
  const { camera } = useThree();

  useEffect(() => {
    camera.lookAt(0, 0.8, 0);
  }, [camera]);

  return (
    <>
      <color attach='background' args={[0x101318]} />
      <ambientLight intensity={0.4} />
      <hemisphereLight args={[0xb7d7ff, 0x29351f, 2.2]} />
      <directionalLight position={[4, 8, 3]} intensity={2.4} />
      <gridHelper args={[12, 12, 0x4f9f78, 0x30423a]} />
      <mesh rotation={[-Math.PI / 2, 0, 0]} position={[0, -0.01, 0]}>
        <planeGeometry args={[12, 12]} />
        <meshStandardMaterial color={0x1c2a25} roughness={0.9} />
      </mesh>
      <LocalAvatar
        room={room}
        localPeerId={localPeerId}
        avatarAssetUrl={avatarAssetUrl}
        chatBubble={latestChatByPeer[localPeerId]}
        onLocalTransform={onLocalTransform}
        onAvatarAssetStatus={onAvatarAssetStatus}
      />
      {remoteEntries.map(([peerId, transform]) => (
        <RemoteAvatar
          key={peerId}
          transform={transform}
          presence={peerPresence[peerId] ?? null}
          chatBubble={latestChatByPeer[peerId]}
          connectionState={connectionState}
          now={now}
        />
      ))}
      <SharedObject object={sharedObject} />
    </>
  );
}

export function MetaverseScene({
  room,
  localPeerId,
  remoteTransforms,
  peerPresence,
  sharedObject,
  avatarAssetUrl,
  latestChatByPeer,
  connectionState,
  now,
  hud,
  onLocalTransform,
  onAvatarAssetStatus,
}: SceneProps) {
  return (
    <div className='metaverse-viewport-shell' aria-label='Metaverse room viewport'>
      <Canvas
        className='metaverse-viewport-canvas'
        camera={{ position: [0, 4.2, 6.5], fov: 58 }}
        gl={{ antialias: true }}
        dpr={[1, 2]}
      >
        <SceneContents
          room={room}
          localPeerId={localPeerId}
          remoteTransforms={remoteTransforms}
          peerPresence={peerPresence}
          sharedObject={sharedObject}
          avatarAssetUrl={avatarAssetUrl}
          latestChatByPeer={latestChatByPeer}
          connectionState={connectionState}
          now={now}
          onLocalTransform={onLocalTransform}
          onAvatarAssetStatus={onAvatarAssetStatus}
        />
      </Canvas>
      {hud}
    </div>
  );
}
