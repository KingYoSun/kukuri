import { useCallback, useEffect, useId, useMemo, useRef, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import type { SupportedLocale } from '@/i18n';
import type {
  GameRoomView,
  DomePhysicsSnapshotV1,
  DomeSessionInputKindV1,
  MetaverseAssetRef,
  MetaverseInteractionKind,
  MetaversePersistentPropV1,
  MetaverseRoomEventView,
  SharedRoomObjectV1,
  SyncStatus,
} from '@/lib/api';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import type { SessionPropView } from '../MetaverseScene';
import { createDomeInteractionInput, persistentPropAsSharedObject } from './DomeSceneModel';
import {
  DEFAULT_SHARED_OBJECT,
  METAVERSE_CHAT_BUBBLE_TTL_MS,
  METAVERSE_ROOM_HEARTBEAT_MS,
  METAVERSE_ROOM_RECOVERY_MS,
  METAVERSE_ROOM_STALE_MS,
  isNewerSharedObject,
  mergeRoomChatMessages,
  normalizeAvatarAnimationState,
  type AvatarTransform,
  type LatestChatBubble,
  type MetaverseRoomConnectionState,
  type MetaverseRoomEvent,
  type MetaverseVec3,
  type PeerPresence,
  type RoomChatMessage,
} from '../MetaverseSceneModel';

type UseMetaverseRoomSessionArgs = {
  actions: MetaverseRoomActions;
  activeTopic: string;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  localDisplayName: string | null;
  localAvatarAssetRef: MetaverseAssetRef | null;
  localAvatarAssetUrl: string | null;
  initialSelectedRoomId?: string | null;
  onError: (message: string | null) => void;
};

const EMPTY_ROOM_CHAT_HISTORY: NonNullable<
  NonNullable<GameRoomView['metaverse']>['chat_history']
> = [];

function chatMessageFromApi(message: {
  room_id: string;
  message_id: string;
  author_peer_id: string;
  display_name?: string | null;
  body: string;
  created_at: number;
}): RoomChatMessage {
  return {
    roomId: message.room_id,
    messageId: message.message_id,
    authorPeerId: message.author_peer_id,
    displayName: message.display_name ?? null,
    body: message.body,
    createdAt: message.created_at,
  };
}

function topicDiagnosticFor(syncStatus: SyncStatus, topic: string) {
  return syncStatus.topic_diagnostics.find(
    (diagnostic) => diagnostic.topic === topic || diagnostic.topic === `hint/${topic}`
  );
}

function latestChatBubbleFromMessage(message: RoomChatMessage, now = Date.now()): LatestChatBubble {
  return {
    peerId: message.authorPeerId,
    displayName: message.displayName ?? null,
    body: message.body,
    createdAt: message.createdAt,
    expiresAt: now + METAVERSE_CHAT_BUBBLE_TTL_MS,
  };
}

export function useMetaverseRoomSession({
  actions,
  activeTopic,
  rooms,
  syncStatus,
  locale,
  localDisplayName,
  localAvatarAssetRef,
  localAvatarAssetUrl,
  initialSelectedRoomId = null,
  onError,
}: UseMetaverseRoomSessionArgs) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [selectedRoomId, setSelectedRoomId] = useState<string | null>(initialSelectedRoomId);
  const [joinedRoomIds, setJoinedRoomIds] = useState<Set<string>>(() => new Set());
  const [remoteTransforms, setRemoteTransforms] = useState<Record<string, AvatarTransform>>({});
  const [peerPresence, setPeerPresence] = useState<Record<string, PeerPresence>>({});
  const [messages, setMessages] = useState<RoomChatMessage[]>([]);
  const [latestChatByPeer, setLatestChatByPeer] = useState<Record<string, LatestChatBubble>>({});
  const [messageDraft, setMessageDraft] = useState('');
  const [sharedObject, setSharedObject] = useState<SharedRoomObjectV1>(DEFAULT_SHARED_OBJECT);
  const [sessionProps, setSessionProps] = useState<SessionPropView[]>([]);
  const [lastSentSeq, setLastSentSeq] = useState(0);
  const [pollErrorCount, setPollErrorCount] = useState(0);
  const [lastRoomActivityAt, setLastRoomActivityAt] = useState(() => Date.now());
  const [recoveringUntil, setRecoveringUntil] = useState(0);
  const [clockNow, setClockNow] = useState(() => Date.now());
  const channelRef = useRef<BroadcastChannel | null>(null);
  const lastBackendEventEnvelopeIdRef = useRef<string | null>(null);
  const lastPhysicsSnapshotSequenceRef = useRef(0);
  const lastRecoveryAtRef = useRef(0);
  const pendingCreatedRoomIdRef = useRef<string | null>(null);
  const sharedObjectRef = useRef<SharedRoomObjectV1>(DEFAULT_SHARED_OBJECT);
  const localPeerSeed = useId().replaceAll(':', '');
  const localPeerId = `${syncStatus.discovery.local_endpoint_id || syncStatus.local_author_pubkey || 'local'}:${localPeerSeed}`;
  const lastSentTransformRef = useRef<AvatarTransform | null>(null);
  const lastReceivedAt = useMemo(() => {
    const values = Object.values(remoteTransforms).map((transform) => transform.sentAt);
    return values.length ? Math.max(...values) : null;
  }, [remoteTransforms]);
  const remoteAnimationSummary = useMemo(
    () =>
      Object.values(remoteTransforms)
        .map((transform) => `${transform.peerId.slice(0, 8)}:${transform.animation}`)
        .join(', '),
    [remoteTransforms]
  );

  const selectedRoom = selectedRoomId
    ? rooms.find((room) => room.room_id === selectedRoomId) ?? null
    : null;
  const selectedRoomRoomId = selectedRoom?.room_id ?? null;
  const selectedRoomSharedObject = selectedRoom?.metaverse
    ? persistentPropAsSharedObject(
        selectedRoom.metaverse.dome.customization.persistent_props[0],
        selectedRoom.host_pubkey,
        selectedRoom.updated_at
      )
    : null;
  const selectedRoomChatHistory = selectedRoom?.metaverse?.chat_history ?? EMPTY_ROOM_CHAT_HISTORY;
  const activeTopicDiagnostic = useMemo(
    () => topicDiagnosticFor(syncStatus, activeTopic),
    [activeTopic, syncStatus]
  );
  const roomConnectionState: MetaverseRoomConnectionState = useMemo(() => {
    if (!selectedRoom) {
      return 'offline';
    }
    if (recoveringUntil > clockNow) {
      return 'recovering';
    }
    const topicPeerCount = activeTopicDiagnostic?.peer_count ?? syncStatus.peer_count;
    const topicError = activeTopicDiagnostic?.last_error ?? syncStatus.last_error ?? null;
    if (
      !syncStatus.connected ||
      syncStatus.delivery_state === 'Offline' ||
      topicPeerCount === 0 ||
      pollErrorCount >= 3 ||
      topicError
    ) {
      return 'offline';
    }
    if (clockNow - lastRoomActivityAt > METAVERSE_ROOM_STALE_MS) {
      return 'stale';
    }
    return 'live';
  }, [
    activeTopicDiagnostic,
    clockNow,
    lastRoomActivityAt,
    pollErrorCount,
    recoveringUntil,
    selectedRoom,
    syncStatus,
  ]);
  const knownPeerCount = Object.keys(remoteTransforms).length;

  useEffect(() => {
    sharedObjectRef.current = sharedObject;
  }, [sharedObject]);

  useEffect(() => {
    if (!selectedRoomId) {
      return;
    }
    if (rooms.some((room) => room.room_id === selectedRoomId)) {
      if (pendingCreatedRoomIdRef.current === selectedRoomId) {
        pendingCreatedRoomIdRef.current = null;
      }
      return;
    }
    if (pendingCreatedRoomIdRef.current !== selectedRoomId) {
      setSelectedRoomId(null);
    }
  }, [rooms, selectedRoomId]);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      const now = Date.now();
      setClockNow(now);
      setLatestChatByPeer((current) => {
        const next = Object.fromEntries(
          Object.entries(current).filter(([, bubble]) => bubble.expiresAt > now)
        );
        return Object.keys(next).length === Object.keys(current).length ? current : next;
      });
    }, 1000);
    return () => {
      window.clearInterval(intervalId);
    };
  }, []);

  useEffect(() => {
    if (!selectedRoomRoomId) {
      return;
    }
    sharedObjectRef.current = DEFAULT_SHARED_OBJECT;
    setSharedObject(DEFAULT_SHARED_OBJECT);
    setSessionProps([]);
    setRemoteTransforms({});
    setPeerPresence({});
    setMessages([]);
    setLatestChatByPeer({});
    setPollErrorCount(0);
    setLastRoomActivityAt(Date.now());
    lastBackendEventEnvelopeIdRef.current = null;
    lastPhysicsSnapshotSequenceRef.current = 0;
    if (typeof BroadcastChannel === 'undefined') {
      return;
    }
    const channel = new BroadcastChannel(`kukuri-metaverse-room:${selectedRoomRoomId}`);
    channelRef.current = channel;
    channel.onmessage = (event: MessageEvent<MetaverseRoomEvent>) => {
      const data = event.data;
      if (!data || !('type' in data)) {
        return;
      }
      setLastRoomActivityAt(Date.now());
      if (data.type === 'presence.join' && data.presence.peerId !== localPeerId) {
        setPeerPresence((current) => ({
          ...current,
          [data.presence.peerId]: data.presence,
        }));
      }
      if (data.type === 'presence.leave' && data.peerId !== localPeerId) {
        setPeerPresence((current) => {
          const next = { ...current };
          delete next[data.peerId];
          return next;
        });
        setRemoteTransforms((current) => {
          const next = { ...current };
          delete next[data.peerId];
          return next;
        });
        setLatestChatByPeer((current) => {
          const next = { ...current };
          delete next[data.peerId];
          return next;
        });
      }
      if (data.type === 'chat.message') {
        setMessages((current) => mergeRoomChatMessages(current, [data.message]));
        setLatestChatByPeer((current) => ({
          ...current,
          [data.message.authorPeerId]: latestChatBubbleFromMessage(data.message),
        }));
      }
    };
    return () => {
      channel.close();
      channelRef.current = null;
    };
  }, [localPeerId, selectedRoomRoomId]);

  useEffect(() => {
    const nextObject = selectedRoomSharedObject ?? DEFAULT_SHARED_OBJECT;
    setSharedObject((current) => {
      if (!isNewerSharedObject(current, nextObject)) {
        return current;
      }
      sharedObjectRef.current = nextObject;
      return nextObject;
    });
  }, [selectedRoomSharedObject]);

  useEffect(() => {
    const durableMessages = selectedRoomChatHistory.map(chatMessageFromApi);
    setMessages((current) => mergeRoomChatMessages(current, durableMessages));
  }, [selectedRoomChatHistory, selectedRoomRoomId]);

  function emit(event: MetaverseRoomEvent) {
    channelRef.current?.postMessage(event);
  }

  const applyPhysicsSnapshot = useCallback((snapshot: DomePhysicsSnapshotV1) => {
    if (snapshot.sequence <= lastPhysicsSnapshotSequenceRef.current) return;
    lastPhysicsSnapshotSequenceRef.current = snapshot.sequence;
    const remote: Record<string, AvatarTransform> = {};
    for (const body of snapshot.bodies) {
      if (body.kind === 'avatar' && body.entity_id !== syncStatus.local_author_pubkey) {
        remote[body.entity_id] = {
          roomId: snapshot.instance_id,
          peerId: body.entity_id,
          seq: snapshot.sequence,
          position: body.position,
          rotation: body.rotation,
          animation: normalizeAvatarAnimationState(body.animation),
          sentAt: snapshot.simulated_at,
        };
      }
    }
    setRemoteTransforms(remote);
    const propBodies = snapshot.bodies.filter(
      (body) => body.kind === 'persistent_prop' || body.kind === 'guest_prop'
    );
    setSessionProps(propBodies.map((body) => {
      const definition = selectedRoom?.metaverse?.dome.customization.persistent_props.find(
        (candidate) => candidate.prop_id === body.entity_id
      );
      return {
        kind: body.kind as SessionPropView['kind'],
        object: {
          object_id: body.entity_id,
          asset_ref: definition?.asset_ref ?? null,
          primitive_fallback: definition?.primitive_fallback ?? 'cube',
          position: body.position,
          rotation: body.rotation,
          scale: definition?.scale ?? [100, 100, 100],
          updated_by: snapshot.host_pubkey,
          updated_at: snapshot.simulated_at,
        },
        collider: definition?.collider ?? null,
      };
    }));
    const prop = propBodies.find((body) => body.kind === 'persistent_prop');
    if (prop) {
      setSharedObject((current) => {
        const next = {
          ...current,
          object_id: prop.entity_id,
          position: prop.position,
          rotation: prop.rotation,
          updated_by: snapshot.host_pubkey,
          updated_at: snapshot.simulated_at,
        };
        sharedObjectRef.current = next;
        return next;
      });
    }
    setLastRoomActivityAt(Date.now());
  }, [selectedRoom, syncStatus.local_author_pubkey]);

  const submitAuthoritativeInput = useCallback((input: DomeSessionInputKindV1, sequence = Date.now()) => {
    if (!selectedRoom?.metaverse) return;
    void actions
      .submitSessionInput(
        selectedRoom.metaverse.spatial_context,
        selectedRoom.metaverse.instance_id,
        sequence,
        input
      )
      .then(applyPhysicsSnapshot)
      .catch(() => {
        // Hosting may not have been started yet; presence/chat remain available.
      });
  }, [actions, applyPhysicsSnapshot, selectedRoom]);

  useEffect(() => {
    if (!selectedRoom) {
      return;
    }
    const joinedAt = Date.now();
    const publishPresence = () => {
      const now = Date.now();
      const presence: PeerPresence = {
        peerId: localPeerId,
        displayName: localDisplayName,
        avatarAssetRef: localAvatarAssetRef,
        avatarAssetUrl: localAvatarAssetUrl,
        joinedAt,
        lastSeenAt: now,
      };
      emit({ type: 'presence.join', presence });
      void actions.publishRoomEvent(selectedRoom.room_id, localPeerId, now, {
        type: 'presence_join',
        presence: {
          room_id: selectedRoom.room_id,
          peer_id: localPeerId,
          display_name: localDisplayName,
          avatar_asset_ref: localAvatarAssetRef,
          joined_at: joinedAt,
          last_seen_at: now,
        },
      }).catch(() => {
        // Browser-only fallback is handled by the local scene.
      });
      submitAuthoritativeInput({ type: 'join' }, now);
    };
    publishPresence();
    const intervalId = window.setInterval(publishPresence, METAVERSE_ROOM_HEARTBEAT_MS);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [
    activeTopic,
    actions,
    localAvatarAssetRef,
    localAvatarAssetUrl,
    localDisplayName,
    localPeerId,
    selectedRoom,
    submitAuthoritativeInput,
  ]);

  useEffect(() => {
    if (!selectedRoom) {
      return;
    }
    let cancelled = false;
    let timeoutId = 0;
    const applyBackendEvent = (view: MetaverseRoomEventView) => {
      const event = view.content.event;
      setLastRoomActivityAt(Date.now());
      if (event.type === 'presence_join' && event.presence.peer_id !== localPeerId) {
        const presence: PeerPresence = {
          peerId: event.presence.peer_id,
          displayName: event.presence.display_name ?? null,
          avatarAssetRef: event.presence.avatar_asset_ref ?? null,
          joinedAt: event.presence.joined_at,
          lastSeenAt: event.presence.last_seen_at,
        };
        setPeerPresence((current) => ({
          ...current,
          [presence.peerId]: {
            ...current[presence.peerId],
            ...presence,
          },
        }));
        if (presence.avatarAssetRef) {
          void actions
            .getBlobPreviewUrl(
              presence.avatarAssetRef.blob_hash,
              presence.avatarAssetRef.mime_type ?? 'model/vrm'
            )
            .then((avatarAssetUrl) => {
              if (!cancelled && avatarAssetUrl) {
                setPeerPresence((current) => ({
                  ...current,
                  [presence.peerId]: {
                    ...current[presence.peerId],
                    avatarAssetUrl,
                  },
                }));
              }
            })
            .catch(() => {
              // Missing remote avatar blobs fall back to the bundled default VRM.
            });
        }
      }
      if (event.type === 'presence_leave' && event.peer_id !== localPeerId) {
        setPeerPresence((current) => {
          const next = { ...current };
          delete next[event.peer_id];
          return next;
        });
        setRemoteTransforms((current) => {
          const next = { ...current };
          delete next[event.peer_id];
          return next;
        });
        setLatestChatByPeer((current) => {
          const next = { ...current };
          delete next[event.peer_id];
          return next;
        });
      }
      if (event.type === 'chat_message') {
        const message = chatMessageFromApi(event.message);
        setMessages((current) => mergeRoomChatMessages(current, [message]));
        setLatestChatByPeer((current) => ({
          ...current,
          [message.authorPeerId]: latestChatBubbleFromMessage(message),
        }));
      }
    };
    const poll = async () => {
      try {
        const events = await actions.listRoomEvents(
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
        if (!cancelled) {
          setPollErrorCount(0);
        }
      } catch {
        if (!cancelled) {
          setPollErrorCount((current) => current + 1);
        }
        // The browser-only dev shell has no Tauri backend. BroadcastChannel remains the local fallback.
      } finally {
        if (!cancelled) {
          timeoutId = window.setTimeout(() => void poll(), 180);
        }
      }
    };
    void poll();
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
    };
  }, [actions, localPeerId, selectedRoom]);

  useEffect(() => {
    if (!selectedRoom || (roomConnectionState !== 'stale' && roomConnectionState !== 'offline')) {
      return;
    }
    const now = Date.now();
    if (now - lastRecoveryAtRef.current < METAVERSE_ROOM_RECOVERY_MS) {
      return;
    }
    lastRecoveryAtRef.current = now;
    lastBackendEventEnvelopeIdRef.current = null;
    lastPhysicsSnapshotSequenceRef.current = 0;
    if (roomConnectionState === 'stale') {
      setRecoveringUntil(now + 3_000);
    }
    void Promise.resolve(actions.refresh()).catch(() => {
      setPollErrorCount((current) => current + 1);
    });
  }, [actions, roomConnectionState, selectedRoom]);

  function resetRoomRuntimeState() {
    setRemoteTransforms({});
    setPeerPresence({});
    setLatestChatByPeer({});
    setPollErrorCount(0);
    setLastRoomActivityAt(Date.now());
    setRecoveringUntil(0);
    setLastSentSeq(0);
    lastSentTransformRef.current = null;
    lastBackendEventEnvelopeIdRef.current = null;
  }

  function joinRoom(roomId: string) {
    setJoinedRoomIds((current) => new Set(current).add(roomId));
    setSelectedRoomId(roomId);
  }

  function selectCreatedRoom(roomId: string) {
    pendingCreatedRoomIdRef.current = roomId;
    joinRoom(roomId);
  }

  function leaveRoom() {
    if (!selectedRoom) {
      return;
    }
    const roomId = selectedRoom.room_id;
    const leftAt = Date.now();
    emit({ type: 'presence.leave', roomId, peerId: localPeerId, leftAt });
    void actions.publishRoomEvent(roomId, localPeerId, leftAt, {
      type: 'presence_leave',
      room_id: roomId,
      peer_id: localPeerId,
      left_at: leftAt,
    }).catch((leaveError) => {
      onError(leaveError instanceof Error ? leaveError.message : t('errors.publishLeaveFailed'));
    });
    submitAuthoritativeInput({ type: 'leave' }, leftAt);
    setJoinedRoomIds((current) => {
      const next = new Set(current);
      next.delete(roomId);
      return next;
    });
    setSelectedRoomId(null);
    resetRoomRuntimeState();
  }

  function handleLocalTransform(transform: AvatarTransform) {
    lastSentTransformRef.current = transform;
    setLastSentSeq(transform.seq);
    submitAuthoritativeInput({
      type: 'move',
      position: transform.position,
      rotation: transform.rotation,
      animation: transform.animation,
    }, transform.seq);
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
      displayName: localDisplayName,
      body: messageDraft.trim(),
      createdAt: Date.now(),
    };
    setMessages((current) => mergeRoomChatMessages(current, [message]));
    setLatestChatByPeer((current) => ({
      ...current,
      [message.authorPeerId]: latestChatBubbleFromMessage(message),
    }));
    setMessageDraft('');
    emit({ type: 'chat.message', message });
    void actions
      .publishRoomEvent(selectedRoom.room_id, localPeerId, Date.now(), {
        type: 'chat_message',
        message: {
          room_id: message.roomId,
          message_id: message.messageId,
          author_peer_id: message.authorPeerId,
          display_name: message.displayName,
          body: message.body,
          created_at: message.createdAt,
        },
      })
      .catch(() => {
        // Browser-only fallback is handled by BroadcastChannel.
      });
  }

  function moveSharedObject(delta: MetaverseVec3) {
    if (!selectedRoom) {
      return;
    }
    const current = sharedObjectRef.current;
    const nextObject: SharedRoomObjectV1 = {
      ...current,
      position: [
        current.position[0] + delta[0],
        current.position[1] + delta[1],
        current.position[2] + delta[2],
      ],
      updated_by: localPeerId,
      updated_at: Date.now(),
    };
    sharedObjectRef.current = nextObject;
    setSharedObject(nextObject);
    submitAuthoritativeInput({
      type: 'push',
      prop_id: current.object_id,
      impulse: delta,
    });
  }

  function interactWithProp(interaction: MetaverseInteractionKind) {
    if (!selectedRoom?.metaverse) return;
    const prop = selectedRoom.metaverse.dome.customization.persistent_props.find(
      (candidate) => candidate.prop_id === sharedObjectRef.current.object_id
    );
    if (!prop || prop.visual_only || !prop.interactions.includes(interaction)) return;

    const input = createDomeInteractionInput(interaction, prop.prop_id, localPeerId);
    const current = sharedObjectRef.current;
    if (input.type === 'sit') {
      const previous = lastSentTransformRef.current;
      handleLocalTransform({
        roomId: selectedRoom.room_id,
        peerId: localPeerId,
        seq: (previous?.seq ?? lastSentSeq) + 1,
        position: [current.position[0], current.position[1] + Math.ceil(current.scale[1] / 2), current.position[2]],
        rotation: previous?.rotation ?? [0, 0, 0],
        animation: 'idle',
        sentAt: input.issuedAt,
      });
      return;
    }

    const authoritativeInput: DomeSessionInputKindV1 = input.type === 'grab'
      ? { type: 'grab', prop_id: prop.prop_id }
      : input.type === 'throw'
        ? { type: 'throw', prop_id: prop.prop_id, impulse: [0, 100, -250] }
        : { type: 'push', prop_id: prop.prop_id, impulse: [0, 0, -50] };
    submitAuthoritativeInput(authoritativeInput, input.issuedAt);
  }

  const submitPropMutation = useCallback(async (input: DomeSessionInputKindV1) => {
    if (!selectedRoom?.metaverse) {
      throw new Error(t('errors.roomRequired'));
    }
    const snapshot = await actions.submitSessionInput(
      selectedRoom.metaverse.spatial_context,
      selectedRoom.metaverse.instance_id,
      Date.now(),
      input
    );
    applyPhysicsSnapshot(snapshot);
  }, [actions, applyPhysicsSnapshot, selectedRoom, t]);

  const newSessionProp = useCallback((kind: 'guest' | 'persistent'): MetaversePersistentPropV1 => ({
    prop_id: `${kind}-${localPeerId}-${Date.now()}`,
    asset_ref: null,
    primitive_fallback: 'cube',
    position: [0, 150, -250],
    rotation: [0, 0, 0],
    scale: [100, 100, 100],
    visual_only: false,
    interactions: ['grab', 'throw', 'push'],
    collider: {
      shape: 'cuboid',
      center: [0, 0, 0],
      half_extents: [50, 50, 50],
    },
  }), [localPeerId]);

  const spawnGuestProp = useCallback(
    () => submitPropMutation({
      type: 'spawn_guest_prop',
      prop: newSessionProp('guest'),
      expires_at: Date.now() + 5 * 60 * 1000,
    }),
    [newSessionProp, submitPropMutation]
  );

  const addPersistentProp = useCallback(
    () => submitPropMutation({
      type: 'upsert_persistent_prop',
      prop: newSessionProp('persistent'),
    }),
    [newSessionProp, submitPropMutation]
  );

  const deletePersistentProp = useCallback(() => {
    const propId = [...sessionProps]
      .reverse()
      .find((prop) => prop.kind === 'persistent_prop')?.object.object_id;
    if (!propId) {
      return Promise.resolve();
    }
    return submitPropMutation({ type: 'delete_persistent_prop', prop_id: propId });
  }, [sessionProps, submitPropMutation]);

  return {
    selectedRoomId,
    joinedRoomIds,
    selectedRoom,
    localPeerId,
    remoteTransforms,
    peerPresence,
    messages,
    latestChatByPeer,
    messageDraft,
    setMessageDraft,
    sharedObject,
    sessionProps,
    lastSentSeq,
    lastReceivedAt,
    remoteAnimationSummary,
    roomConnectionState,
    knownPeerCount,
    clockNow,
    joinRoom,
    selectCreatedRoom,
    leaveRoom,
    handleLocalTransform,
    handleSendMessage,
    moveSharedObject,
    interactWithProp,
    spawnGuestProp,
    addPersistentProp,
    deletePersistentProp,
  };
}
