import { useEffect, useId, useMemo, useRef, useState, type FormEvent } from 'react';
import { Box, Cuboid, MessageSquare, Move3D, Play, Send } from 'lucide-react';
import * as THREE from 'three';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { VRMLoaderPlugin, VRMUtils, type VRM } from '@pixiv/three-vrm';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';
import type {
  ChannelRef,
  DesktopApi,
  GameRoomView,
  MetaverseAssetRef,
  MetaverseRoomEventView,
  MetaverseRoomEventV1,
  SharedRoomObjectV1,
  SyncStatus,
} from '@/lib/api';
import { formatLocalizedTime } from '@/i18n/format';
import type { SupportedLocale } from '@/i18n';
import { blobToBase64 } from '@/lib/attachments';

type MetaverseVec3 = [number, number, number];

type AvatarTransform = {
  roomId: string;
  peerId: string;
  seq: number;
  position: MetaverseVec3;
  rotation: MetaverseVec3;
  animation: 'idle' | 'walk';
  sentAt: number;
};

type RoomChatMessage = {
  roomId: string;
  messageId: string;
  authorPeerId: string;
  body: string;
  createdAt: number;
};

type MetaverseRoomEvent =
  | { type: 'presence.join'; roomId: string; peerId: string; at: number }
  | { type: 'avatar.transform'; transform: AvatarTransform }
  | { type: 'chat.message'; message: RoomChatMessage }
  | { type: 'object.update'; roomId: string; object: SharedRoomObjectV1 };

type SceneProps = {
  room: GameRoomView;
  localPeerId: string;
  remoteTransforms: Record<string, AvatarTransform>;
  sharedObject: SharedRoomObjectV1;
  avatarAssetUrl: string | null;
  onLocalTransform: (transform: AvatarTransform) => void;
  onAvatarAssetStatus: (status: AvatarAssetStatus) => void;
};

type MetaverseRoomPanelProps = {
  api: DesktopApi;
  activeTopic: string;
  activeComposeChannel: ChannelRef;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  onRefresh: () => Promise<void>;
};

const DEFAULT_SHARED_OBJECT: SharedRoomObjectV1 = {
  object_id: 'mvp-object-1',
  asset_ref: null,
  primitive_fallback: 'cube',
  position: [0, 50, -240],
  rotation: [0, 0, 0],
  scale: [100, 100, 100],
  updated_by: '',
  updated_at: 0,
};

const DEFAULT_AVATAR_ASSET_URL = '/avatar_sample_a.vrm';

type AvatarAssetStatus = 'loading' | 'sample-vrm' | 'blob-vrm' | 'fallback-primitive';

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

function MetaverseScene({
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

    const handleKeyDown = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) {
        return;
      }
      if (
        event.key === 'w' ||
        event.key === 'a' ||
        event.key === 's' ||
        event.key === 'd' ||
        event.key.startsWith('Arrow')
      ) {
        keysRef.current.add(event.key.toLowerCase());
      }
    };
    const handleKeyUp = (event: KeyboardEvent) => {
      keysRef.current.delete(event.key.toLowerCase());
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
      const x = (keys.has('d') || keys.has('arrowright') ? 1 : 0) - (keys.has('a') || keys.has('arrowleft') ? 1 : 0);
      const z = (keys.has('s') || keys.has('arrowdown') ? 1 : 0) - (keys.has('w') || keys.has('arrowup') ? 1 : 0);
      const moving = x !== 0 || z !== 0;
      if (moving) {
        const length = Math.hypot(x, z) || 1;
        const speed = 220;
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
          animation: 'walk',
          sentAt: Date.now(),
        };
        localAvatar.position.copy(scenePosition(nextPosition));
        localAvatar.rotation.y = THREE.MathUtils.degToRad(yaw);
      } else if (localTransformRef.current.animation !== 'idle') {
        localTransformRef.current = {
          ...localTransformRef.current,
          seq: seqRef.current,
          animation: 'idle',
          sentAt: Date.now(),
        };
      }
      avatarAssetRuntime.vrm?.update(deltaSeconds);

      if (frameAt - lastSentAtRef.current >= 100) {
        seqRef.current += 1;
        lastSentAtRef.current = frameAt;
        onLocalTransformRef.current({
          ...localTransformRef.current,
          seq: seqRef.current,
          sentAt: Date.now(),
        });
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

export function MetaverseRoomPanel({
  api,
  activeTopic,
  activeComposeChannel,
  rooms,
  syncStatus,
  locale,
  onRefresh,
}: MetaverseRoomPanelProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [maxPeers, setMaxPeers] = useState('8');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [selectedRoomId, setSelectedRoomId] = useState<string | null>(rooms[0]?.room_id ?? null);
  const [remoteTransforms, setRemoteTransforms] = useState<Record<string, AvatarTransform>>({});
  const [messages, setMessages] = useState<RoomChatMessage[]>([]);
  const [messageDraft, setMessageDraft] = useState('');
  const [sharedObject, setSharedObject] = useState<SharedRoomObjectV1>(
    rooms[0]?.metaverse?.scene.shared_object ?? DEFAULT_SHARED_OBJECT
  );
  const [lastSentSeq, setLastSentSeq] = useState(0);
  const [avatarAssetStatus, setAvatarAssetStatus] = useState<AvatarAssetStatus>('loading');
  const [localAvatarAssetRef, setLocalAvatarAssetRef] = useState<MetaverseAssetRef | null>(null);
  const [localAvatarAssetUrl, setLocalAvatarAssetUrl] = useState<string | null>(null);
  const channelRef = useRef<BroadcastChannel | null>(null);
  const lastBackendEventEnvelopeIdRef = useRef<string | null>(null);
  const localPeerSeed = useId().replaceAll(':', '');
  const localPeerId = `${syncStatus.discovery.local_endpoint_id || syncStatus.local_author_pubkey || 'local'}:${localPeerSeed}`;
  const lastSentTransformRef = useRef<AvatarTransform | null>(null);
  const lastReceivedAt = useMemo(() => {
    const values = Object.values(remoteTransforms).map((transform) => transform.sentAt);
    return values.length ? Math.max(...values) : null;
  }, [remoteTransforms]);

  const selectedRoom =
    rooms.find((room) => room.room_id === selectedRoomId) ?? rooms[0] ?? null;
  const knownPeerCount = Object.keys(remoteTransforms).length;

  useEffect(() => {
    if (!selectedRoom && rooms[0]) {
      setSelectedRoomId(rooms[0].room_id);
    }
  }, [rooms, selectedRoom]);

  useEffect(() => {
    if (!selectedRoom) {
      return;
    }
    setSharedObject(selectedRoom.metaverse?.scene.shared_object ?? DEFAULT_SHARED_OBJECT);
    setRemoteTransforms({});
    setMessages([]);
    lastBackendEventEnvelopeIdRef.current = null;
    if (typeof BroadcastChannel === 'undefined') {
      return;
    }
    const channel = new BroadcastChannel(`kukuri-metaverse-room:${selectedRoom.room_id}`);
    channelRef.current = channel;
    const publish = (event: MetaverseRoomEvent) => channel.postMessage(event);
    channel.onmessage = (event: MessageEvent<MetaverseRoomEvent>) => {
      const data = event.data;
      if (!data || !('type' in data)) {
        return;
      }
      if (data.type === 'avatar.transform' && data.transform.peerId !== localPeerId) {
        setRemoteTransforms((current) => ({
          ...current,
          [data.transform.peerId]: data.transform,
        }));
      }
      if (data.type === 'chat.message' && data.message.authorPeerId !== localPeerId) {
        setMessages((current) => [...current, data.message].slice(-64));
      }
      if (data.type === 'object.update' && data.object.updated_by !== localPeerId) {
        setSharedObject(data.object);
      }
    };
    publish({ type: 'presence.join', roomId: selectedRoom.room_id, peerId: localPeerId, at: Date.now() });
    return () => {
      channel.close();
      channelRef.current = null;
    };
  }, [localPeerId, selectedRoom]);

  useEffect(() => {
    if (!selectedRoom) {
      return;
    }
    const now = Date.now();
    void api.publishMetaverseRoomEvent(activeTopic, selectedRoom.room_id, localPeerId, now, {
      type: 'presence_join',
      presence: {
        room_id: selectedRoom.room_id,
        peer_id: localPeerId,
        display_name: null,
        avatar_asset_ref: localAvatarAssetRef,
        joined_at: now,
        last_seen_at: now,
      },
    }).catch(() => {
      // Browser-only fallback is handled by the local scene.
    });
  }, [activeTopic, api, localAvatarAssetRef, localPeerId, selectedRoom]);

  async function importAvatarBlob(blob: Blob, name: string) {
    if (!selectedRoom) {
      return;
    }
    setPending(true);
    try {
      const mime = blob.type || 'model/vrm';
      const dataBase64 = await blobToBase64(blob);
      const assetRef = await api.importMetaverseRoomAsset(
        activeTopic,
        selectedRoom.room_id,
        'vrm',
        mime,
        name,
        dataBase64
      );
      const resolvedUrl =
        (await api.getBlobPreviewUrl(assetRef.blob_hash, assetRef.mime_type ?? mime)) ??
        `data:${mime};base64,${dataBase64}`;
      setLocalAvatarAssetRef(assetRef);
      setLocalAvatarAssetUrl(resolvedUrl);
      setError(null);
    } catch (assetError) {
      setError(assetError instanceof Error ? assetError.message : 'Failed to import VRM avatar');
    } finally {
      setPending(false);
    }
  }

  async function handleSampleAvatarImport() {
    const response = await fetch(DEFAULT_AVATAR_ASSET_URL);
    if (!response.ok) {
      throw new Error(`sample VRM fetch failed: ${response.status}`);
    }
    await importAvatarBlob(await response.blob(), 'avatar_sample_a.vrm');
  }

  useEffect(() => {
    if (!selectedRoom) {
      return;
    }
    let cancelled = false;
    let timeoutId = 0;
    const applyBackendEvent = (view: MetaverseRoomEventView) => {
      const event = view.content.event;
      if (event.type === 'avatar_transform' && event.transform.peer_id !== localPeerId) {
        setRemoteTransforms((current) => ({
          ...current,
          [event.transform.peer_id]: {
            roomId: event.transform.room_id,
            peerId: event.transform.peer_id,
            seq: event.transform.seq,
            position: event.transform.position,
            rotation: event.transform.rotation,
            animation: event.transform.animation === 'walk' ? 'walk' : 'idle',
            sentAt: event.transform.sent_at,
          },
        }));
      }
      if (event.type === 'chat_message' && event.message.author_peer_id !== localPeerId) {
        setMessages((current) => [
          ...current,
          {
            roomId: event.message.room_id,
            messageId: event.message.message_id,
            authorPeerId: event.message.author_peer_id,
            body: event.message.body,
            createdAt: event.message.created_at,
          },
        ].slice(-64));
      }
      if (event.type === 'object_update' && event.object.updated_by !== localPeerId) {
        setSharedObject(event.object);
      }
    };
    const poll = async () => {
      try {
        const events = await api.listMetaverseRoomEvents(
          activeTopic,
          selectedRoom.room_id,
          lastBackendEventEnvelopeIdRef.current,
          64
        );
        if (!cancelled && events.length > 0) {
          for (const event of events) {
            applyBackendEvent(event);
          }
          lastBackendEventEnvelopeIdRef.current = events[events.length - 1].envelope_id;
        }
      } catch {
        // The browser-only dev shell has no Tauri backend. BroadcastChannel remains the local fallback.
      } finally {
        if (!cancelled) {
          timeoutId = window.setTimeout(() => void poll(), 600);
        }
      }
    };
    void poll();
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
    };
  }, [activeTopic, api, localPeerId, selectedRoom]);

  async function handleCreateRoom(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!title.trim()) {
      setError('Room title is required');
      return;
    }
    setPending(true);
    try {
      const parsedMaxPeers = Number.parseInt(maxPeers, 10);
      const roomId = await api.createMetaverseRoom(
        activeTopic,
        title.trim(),
        description.trim(),
        Number.isNaN(parsedMaxPeers) ? null : parsedMaxPeers,
        activeComposeChannel
      );
      setTitle('');
      setDescription('');
      setMaxPeers('8');
      setError(null);
      setSelectedRoomId(roomId);
      await onRefresh();
    } catch (createError) {
      setError(createError instanceof Error ? createError.message : 'Failed to create metaverse room');
    } finally {
      setPending(false);
    }
  }

  function emit(event: MetaverseRoomEvent) {
    channelRef.current?.postMessage(event);
  }

  function handleLocalTransform(transform: AvatarTransform) {
    lastSentTransformRef.current = transform;
    setLastSentSeq(transform.seq);
    emit({ type: 'avatar.transform', transform });
    const event: MetaverseRoomEventV1 = {
      type: 'avatar_transform',
      transform: {
        room_id: transform.roomId,
        peer_id: transform.peerId,
        seq: transform.seq,
        position: transform.position,
        rotation: transform.rotation,
        animation: transform.animation,
        sent_at: transform.sentAt,
      },
    };
    void api.publishMetaverseRoomEvent(activeTopic, transform.roomId, localPeerId, transform.seq, event).catch(() => {
      // Browser-only fallback is handled by BroadcastChannel.
    });
  }

  function handleSendMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedRoom || !messageDraft.trim()) {
      return;
    }
    const message: RoomChatMessage = {
      roomId: selectedRoom.room_id,
      messageId: `${localPeerId}-${Date.now()}`,
      authorPeerId: localPeerId,
      body: messageDraft.trim(),
      createdAt: Date.now(),
    };
    setMessages((current) => [...current, message].slice(-64));
    setMessageDraft('');
    emit({ type: 'chat.message', message });
    void api.publishMetaverseRoomEvent(
      activeTopic,
      selectedRoom.room_id,
      localPeerId,
      Date.now(),
      {
        type: 'chat_message',
        message: {
          room_id: message.roomId,
          message_id: message.messageId,
          author_peer_id: message.authorPeerId,
          display_name: null,
          body: message.body,
          created_at: message.createdAt,
        },
      }
    ).catch(() => {
      // Browser-only fallback is handled by BroadcastChannel.
    });
  }

  async function moveSharedObject(delta: MetaverseVec3) {
    if (!selectedRoom) {
      return;
    }
    const nextObject: SharedRoomObjectV1 = {
      ...sharedObject,
      position: [
        sharedObject.position[0] + delta[0],
        sharedObject.position[1] + delta[1],
        sharedObject.position[2] + delta[2],
      ],
      updated_by: localPeerId,
      updated_at: Date.now(),
    };
    setSharedObject(nextObject);
    emit({ type: 'object.update', roomId: selectedRoom.room_id, object: nextObject });
    void api.publishMetaverseRoomEvent(
      activeTopic,
      selectedRoom.room_id,
      localPeerId,
      Date.now(),
      {
        type: 'object_update',
        object: nextObject,
      }
    ).catch(() => {
      // Browser-only fallback is handled by BroadcastChannel.
    });
    try {
      await api.updateMetaverseRoom(
        activeTopic,
        selectedRoom.room_id,
        selectedRoom.status,
        nextObject.position,
        nextObject.rotation,
        nextObject.scale
      );
      await onRefresh();
    } catch (updateError) {
      setError(updateError instanceof Error ? updateError.message : 'Failed to persist shared object');
    }
  }

  return (
    <div className='metaverse-panel'>
      <Card className='shell-workspace-card metaverse-discovery-card'>
        <div className='panel-header'>
          <div>
            <h3>Metaverse Rooms</h3>
            <small>{rooms.length} room{rooms.length === 1 ? '' : 's'} in this topic</small>
          </div>
        </div>
        {error ? <Notice tone='destructive'>{error}</Notice> : null}
        <form className='composer composer-compact metaverse-create-form' onSubmit={handleCreateRoom}>
          <Label>
            <span>Room title</span>
            <Input
              value={title}
              placeholder='Atrium'
              disabled={pending}
              onChange={(event) => setTitle(event.target.value)}
            />
          </Label>
          <Label>
            <span>Description</span>
            <Textarea
              value={description}
              placeholder='Small social space'
              disabled={pending}
              onChange={(event) => setDescription(event.target.value)}
            />
          </Label>
          <Label>
            <span>Max peers</span>
            <Input
              value={maxPeers}
              disabled={pending}
              onChange={(event) => setMaxPeers(event.target.value)}
            />
          </Label>
          <Button type='submit' disabled={pending}>
            <Cuboid className='size-4' aria-hidden='true' />
            Create metaverse room
          </Button>
        </form>
        {rooms.length === 0 ? <p className='empty-state'>No metaverse rooms in this topic.</p> : null}
        <ul className='metaverse-room-grid'>
          {rooms.map((room) => (
            <li key={room.room_id}>
              <article className={`metaverse-room-card${selectedRoom?.room_id === room.room_id ? ' metaverse-room-card-active' : ''}`}>
                <div className='post-meta'>
                  <span>{room.title}</span>
                  <span>{room.status}</span>
                  <span className='reply-chip'>{room.audience_label}</span>
                </div>
                <p>{room.description || 'No description'}</p>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>Host: {room.host_pubkey.slice(0, 10)}</span>
                  <span>Updated: {formatLocalizedTime(room.updated_at, locale)}</span>
                </div>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>Manifest: {room.manifest_blob_hash ?? 'pending'}</span>
                  <span>World: {room.metaverse?.world_version ?? 1}</span>
                </div>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() => setSelectedRoomId(room.room_id)}
                >
                  <Play className='size-4' aria-hidden='true' />
                  Open room
                </Button>
              </article>
            </li>
          ))}
        </ul>
      </Card>

      {selectedRoom ? (
        <Card className='shell-workspace-card metaverse-room-view'>
          <div className='metaverse-room-stage'>
            <MetaverseScene
              room={selectedRoom}
              localPeerId={localPeerId}
              remoteTransforms={remoteTransforms}
              sharedObject={sharedObject}
              avatarAssetUrl={localAvatarAssetUrl}
              onLocalTransform={handleLocalTransform}
              onAvatarAssetStatus={setAvatarAssetStatus}
            />
            <aside className='metaverse-room-sidebar'>
              <div className='panel-header'>
                <div>
                  <h3>{selectedRoom.title}</h3>
                  <small>{selectedRoom.room_id}</small>
                </div>
              </div>
              <div className='topic-diagnostic'>
                <span>Topic: {activeTopic}</span>
                <span>Local peer: {localPeerId}</span>
                <span>Known peers: {knownPeerCount}</span>
                <span>Last sent seq: {lastSentSeq}</span>
                <span>Last received: {lastReceivedAt ? formatLocalizedTime(lastReceivedAt, locale) : 'none'}</span>
                <span>
                  Avatar asset:{' '}
                  {avatarAssetStatus === 'sample-vrm'
                    ? 'sample VRM loaded'
                    : avatarAssetStatus === 'blob-vrm'
                      ? 'blob VRM loaded'
                      : avatarAssetStatus}
                </span>
                <span>Blob asset resolve: {localAvatarAssetRef?.blob_hash ?? 'public sample / fallback-ready'}</span>
                <span>Persistence: manifest blob {selectedRoom.manifest_blob_hash ?? 'pending'}</span>
                <span>Community assist: {syncStatus.discovery.bootstrap_seed_peer_ids.length > 0 ? 'available' : 'optional'}</span>
              </div>
              <div className='metaverse-object-controls'>
                <strong>Avatar asset</strong>
                <div className='metaverse-nudge-grid'>
                  <Button variant='secondary' type='button' disabled={pending} onClick={() => void handleSampleAvatarImport()}>
                    Sample VRM
                  </Button>
                  <Label>
                    <span>VRM file</span>
                    <Input
                      type='file'
                      accept='.vrm,model/vrm,application/octet-stream'
                      disabled={pending}
                      onChange={(event) => {
                        const file = event.target.files?.[0];
                        if (file) {
                          void importAvatarBlob(file, file.name);
                        }
                        event.currentTarget.value = '';
                      }}
                    />
                  </Label>
                </div>
              </div>
              <div className='metaverse-object-controls'>
                <strong>
                  <Box className='size-4' aria-hidden='true' />
                  Shared object
                </strong>
                <div className='metaverse-nudge-grid'>
                  <Button variant='secondary' type='button' onClick={() => void moveSharedObject([0, 0, -50])}>
                    <Move3D className='size-4' aria-hidden='true' />
                    Forward
                  </Button>
                  <Button variant='secondary' type='button' onClick={() => void moveSharedObject([-50, 0, 0])}>Left</Button>
                  <Button variant='secondary' type='button' onClick={() => void moveSharedObject([50, 0, 0])}>Right</Button>
                  <Button variant='secondary' type='button' onClick={() => void moveSharedObject([0, 0, 50])}>Back</Button>
                </div>
              </div>
              <form className='metaverse-chat-form' onSubmit={handleSendMessage}>
                <Label>
                  <span>
                    <MessageSquare className='size-4' aria-hidden='true' />
                    Room chat
                  </span>
                  <Input
                    value={messageDraft}
                    placeholder='Say something in the room'
                    onChange={(event) => setMessageDraft(event.target.value)}
                  />
                </Label>
                <Button type='submit'>
                  <Send className='size-4' aria-hidden='true' />
                  Send
                </Button>
              </form>
              <ul className='metaverse-chat-list'>
                {messages.map((message) => (
                  <li key={message.messageId}>
                    <strong>
                      {message.authorPeerId === localPeerId ? 'You' : message.authorPeerId.slice(0, 12)}
                      <small>{formatLocalizedTime(message.createdAt, locale)}</small>
                    </strong>
                    <span>{message.body}</span>
                  </li>
                ))}
              </ul>
            </aside>
          </div>
        </Card>
      ) : null}
    </div>
  );
}
