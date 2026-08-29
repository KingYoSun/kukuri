import { useCallback, useEffect, useId, useMemo, useRef, useState, type FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import type { SupportedLocale } from '@/i18n';
import type {
  DomeBoundaryStateV1,
  DomeDirection,
  GameRoomView,
  DomePhysicsSnapshotV1,
  DomeSessionInputKindV1,
  DomeTransitionAdmissionTicketV1,
  MetaverseAssetRef,
  MetaverseColliderV1,
  MetaverseInteractionKind,
  MetaversePersistentPropV1,
  SharedRoomObjectV1,
  SpatialContextV1,
  SyncStatus,
} from '@/lib/api';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import type { SessionPropView } from '../MetaverseScene';
import { createDomeInteractionInput, persistentPropAsSharedObject } from './DomeSceneModel';
import {
  domeTransitionProgress,
  transitionNeighborAtPosition,
  transitionNeighborInZone,
  transformAvatarBetweenDomes,
  type DomeNeighborTransitionView,
} from './DomeTransitionModel';
import { useDomeTransitionNeighbors } from './useDomeTransitionNeighbors';
import {
  DEFAULT_SHARED_OBJECT,
  METAVERSE_ROOM_HEARTBEAT_MS,
  METAVERSE_ROOM_RECOVERY_MS,
  METAVERSE_ROOM_STALE_MS,
  isNewerSharedObject,
  mergeRoomChatMessages,
  normalizeAvatarAnimationState,
  type AvatarTransform,
  type MetaverseRoomConnectionState,
  type MetaverseRoomEvent,
  type MetaverseVec3,
  type PeerPresence,
  type RoomChatMessage,
} from '../MetaverseSceneModel';
import {
  chatMessageFromApi,
  latestChatBubbleFromMessage,
  topicDiagnosticFor,
} from './MetaverseRoomSessionSupport';
import { useSpatialAudio } from './useSpatialAudio';
import { useMetaverseBackendEvents } from './useMetaverseBackendEvents';
import {
  readLastVisitedDome,
  resolveDomeEntryOrder,
  spatialContextKey,
  writeLastVisitedDome,
} from './DomeEntryModel';
import { loadAvatarCollider } from './AvatarColliderModel';

type UseMetaverseRoomSessionArgs = {
  actions: MetaverseRoomActions;
  activeTopic: string;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  localDisplayName: string | null;
  localAvatarAssetRef: MetaverseAssetRef | null;
  localAvatarAssetUrl: string | null;
  mutedAuthorPubkeys?: ReadonlySet<string>;
  initialSelectedRoomId?: string | null;
  activeChannelId?: string | null;
  configuredEntryInstanceId?: string | null;
  onError: (message: string | null) => void;
};

type TransitionAttempt = {
  id: string;
  sourceRoom: GameRoomView;
  neighbor: DomeNeighborTransitionView;
  phase: 'preparing' | 'provisional' | 'committing' | 'target_committed';
  ticket: DomeTransitionAdmissionTicketV1 | null;
  cancelled: boolean;
};

export type DomeRecoveryStatus = {
  state: 'online' | 'offline' | 'evacuating' | 'closed' | 'no_candidate';
  secondsRemaining: number | null;
  reason: 'host_offline' | 'access_revoked' | 'blocked' | 'user_requested' | null;
  targetTitle: string | null;
};

export const ONLINE_DOME_RECOVERY: DomeRecoveryStatus = {
  state: 'online',
  secondsRemaining: null,
  reason: null,
  targetTitle: null,
};

const EMPTY_MUTED_AUTHOR_PUBKEYS = new Set<string>();

const EMPTY_ROOM_CHAT_HISTORY: NonNullable<
  NonNullable<GameRoomView['metaverse']>['chat_history']
> = [];

export function useMetaverseRoomSession({
  actions,
  activeTopic,
  rooms,
  syncStatus,
  locale,
  localDisplayName,
  localAvatarAssetRef,
  localAvatarAssetUrl,
  mutedAuthorPubkeys = EMPTY_MUTED_AUTHOR_PUBKEYS,
  initialSelectedRoomId = null,
  activeChannelId = null,
  configuredEntryInstanceId = null,
  onError,
}: UseMetaverseRoomSessionArgs) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [selectedRoomId, setSelectedRoomId] = useState<string | null>(initialSelectedRoomId);
  const [admissionConfirmed, setAdmissionConfirmed] = useState(false);
  const [admissionStatus, setAdmissionStatus] = useState<'resolving' | 'admitting' | 'joined' | 'selection'>('resolving');
  const [joinedRoomIds, setJoinedRoomIds] = useState<Set<string>>(() => new Set());
  const [remoteTransforms, setRemoteTransforms] = useState<Record<string, AvatarTransform>>({});
  const [messageDraft, setMessageDraft] = useState('');
  const [sharedObject, setSharedObject] = useState<SharedRoomObjectV1>(DEFAULT_SHARED_OBJECT);
  const [sessionProps, setSessionProps] = useState<SessionPropView[]>([]);
  const [transitionPreparingDirections, setTransitionPreparingDirections] = useState<Set<DomeDirection>>(
    () => new Set()
  );
  const [handoffTransform, setHandoffTransform] = useState<AvatarTransform | null>(null);
  const [lastSentSeq, setLastSentSeq] = useState(0);
  const [recoveringUntil, setRecoveringUntil] = useState(0);
  const [clockNow, setClockNow] = useState(() => Date.now());
  const [domeRecovery, setDomeRecovery] = useState<DomeRecoveryStatus>(ONLINE_DOME_RECOVERY);
  const [pendingEvacuationReason, setPendingEvacuationReason] = useState<
    'access_revoked' | 'blocked' | null
  >(null);
  const channelRef = useRef<BroadcastChannel | null>(null);
  const lastPhysicsSnapshotSequenceRef = useRef(0);
  const lastRecoveryAtRef = useRef(0);
  const pendingCreatedRoomIdRef = useRef<string | null>(null);
  const sharedObjectRef = useRef<SharedRoomObjectV1>(DEFAULT_SHARED_OBJECT);
  const localPeerSeed = useId().replaceAll(':', '');
  const localPeerId = `${syncStatus.discovery.local_endpoint_id || syncStatus.local_author_pubkey || 'local'}:${localPeerSeed}`;
  const lastSentTransformRef = useRef<AvatarTransform | null>(null);
  const transitionAttemptRef = useRef<TransitionAttempt | null>(null);
  const entryAttemptKeyRef = useRef<string | null>(null);
  const entryContextKeyRef = useRef<string | null>(null);
  const entryAutoDisabledRef = useRef(false);
  const evacuationRunningRef = useRef(false);
  const avatarColliderPromiseRef = useRef<{
    assetUrl: string | null;
    promise: Promise<MetaverseColliderV1 | null>;
  } | null>(null);
  const sessionSequenceByInstanceRef = useRef(new Map<string, number>());
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
  const admittedRoom = admissionConfirmed ? selectedRoom : null;
  const entryContext = useMemo<SpatialContextV1>(() => activeChannelId
    ? { kind: 'channel', topic_id: activeTopic, channel_id: activeChannelId }
    : { kind: 'topic', topic_id: activeTopic }, [activeChannelId, activeTopic]);
  const entryCandidates = useMemo(() => resolveDomeEntryOrder({
    rooms,
    localAuthorPubkey: syncStatus.local_author_pubkey,
    lastVisitedInstanceId: readLastVisitedDome(syncStatus.local_author_pubkey, entryContext),
    configuredEntryInstanceId,
  }), [configuredEntryInstanceId, entryContext, rooms, syncStatus.local_author_pubkey]);
  const [transitionNeighbors, setTransitionNeighbors] = useDomeTransitionNeighbors(
    actions,
    admittedRoom,
    rooms,
    syncStatus.local_author_pubkey
  );

  const {
    microphoneEnabled,
    toggleMicrophone,
    disableMicrophone,
    playSpatialAudioFrame,
    stopSpatialAudio,
  } = useSpatialAudio({
    actions,
    selectedRoom: admittedRoom,
    localPeerId,
    listenerTransformRef: lastSentTransformRef,
    mutedAuthorPubkeys,
    transitionNeighbors,
  });
  const {
    peerPresence,
    setPeerPresence,
    messages,
    setMessages,
    latestChatByPeer,
    setLatestChatByPeer,
    pollErrorCount,
    setPollErrorCount,
    lastRoomActivityAt,
    setLastRoomActivityAt,
    resetBackendEventCursor,
  } = useMetaverseBackendEvents({
    actions,
    selectedRoom: admittedRoom,
    transitionNeighbors,
    localPeerId,
    playSpatialAudioFrame,
    setRemoteTransforms,
  });
  const selectedRoomRoomId = admittedRoom?.room_id ?? null;
  const selectedRoomSharedObject = admittedRoom?.metaverse
    ? persistentPropAsSharedObject(
        admittedRoom.metaverse.dome.customization.persistent_props[0],
        admittedRoom.host_pubkey,
        admittedRoom.updated_at
      )
    : null;
  const selectedRoomChatHistory = admittedRoom?.metaverse?.chat_history ?? EMPTY_ROOM_CHAT_HISTORY;
  const activeTopicDiagnostic = useMemo(
    () => topicDiagnosticFor(syncStatus, activeTopic),
    [activeTopic, syncStatus]
  );
  const roomConnectionState: MetaverseRoomConnectionState = useMemo(() => {
    if (!admittedRoom) {
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
    admittedRoom,
    syncStatus,
  ]);
  const knownPeerCount = Object.keys(remoteTransforms).length;
  const transitionBoundaryStates = useMemo(() => {
    const states: Partial<Record<DomeDirection, DomeBoundaryStateV1>> = {};
    for (const neighbor of transitionNeighbors) {
      states[neighbor.direction] = domeRecovery.state === 'offline'
        ? 'offline'
        : transitionPreparingDirections.has(neighbor.direction)
        ? 'loading'
        : neighbor.boundaryState;
    }
    return states;
  }, [domeRecovery.state, transitionNeighbors, transitionPreparingDirections]);

  const nextSessionSequence = useCallback((instanceId: string, suggested = Date.now()) => {
    const next = Math.max(suggested, (sessionSequenceByInstanceRef.current.get(instanceId) ?? 0) + 1);
    sessionSequenceByInstanceRef.current.set(instanceId, next);
    return next;
  }, []);

  const submitInputForRoom = useCallback((
    room: GameRoomView,
    input: DomeSessionInputKindV1,
    suggestedSequence = Date.now()
  ) => {
    if (!room.metaverse) return Promise.reject(new Error('Dome room state is unavailable'));
    return actions.submitSessionInput(
      room.metaverse.spatial_context,
      room.metaverse.instance_id,
      nextSessionSequence(room.metaverse.instance_id, suggestedSequence),
      input
    );
  }, [actions, nextSessionSequence]);

  const resolveLocalAvatarCollider = useCallback(() => {
    if (avatarColliderPromiseRef.current?.assetUrl !== localAvatarAssetUrl) {
      avatarColliderPromiseRef.current = {
        assetUrl: localAvatarAssetUrl,
        promise: loadAvatarCollider(localAvatarAssetUrl).catch(() => null),
      };
    }
    return avatarColliderPromiseRef.current.promise;
  }, [localAvatarAssetUrl]);

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
      setAdmissionConfirmed(false);
      setAdmissionStatus('selection');
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
  }, [setLatestChatByPeer]);

  useEffect(() => {
    sharedObjectRef.current = DEFAULT_SHARED_OBJECT;
    setSharedObject(DEFAULT_SHARED_OBJECT);
    setSessionProps([]);
    setRemoteTransforms({});
    setPeerPresence({});
    setMessages([]);
    setLatestChatByPeer({});
    setPollErrorCount(0);
    resetBackendEventCursor();
    lastPhysicsSnapshotSequenceRef.current = 0;
  }, [
    resetBackendEventCursor,
    selectedRoom?.room_id,
    setLatestChatByPeer,
    setMessages,
    setPeerPresence,
    setPollErrorCount,
  ]);

  useEffect(() => {
    if (!selectedRoomRoomId) {
      return;
    }
    setLastRoomActivityAt(Date.now());
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
  }, [
    localPeerId,
    resetBackendEventCursor,
    selectedRoomRoomId,
    setLastRoomActivityAt,
    setLatestChatByPeer,
    setMessages,
    setPeerPresence,
    setPollErrorCount,
  ]);

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
  }, [selectedRoomChatHistory, selectedRoomRoomId, setMessages]);

  function emit(event: MetaverseRoomEvent) {
    channelRef.current?.postMessage(event);
  }

  const applyPhysicsSnapshot = useCallback((snapshot: DomePhysicsSnapshotV1) => {
    if (snapshot.sequence <= lastPhysicsSnapshotSequenceRef.current) return;
    lastPhysicsSnapshotSequenceRef.current = snapshot.sequence;
    const remote: Record<string, AvatarTransform> = {};
    for (const body of snapshot.bodies) {
      if (
        body.kind === 'avatar'
        && body.entity_id !== `avatar:${syncStatus.local_author_pubkey}`
      ) {
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
      const definition = admittedRoom?.metaverse?.dome.customization.persistent_props.find(
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
  }, [admittedRoom, setLastRoomActivityAt, syncStatus.local_author_pubkey]);

  const submitAuthoritativeInput = useCallback((input: DomeSessionInputKindV1, sequence = Date.now()) => {
    if (!admittedRoom?.metaverse || domeRecovery.state !== 'online') return;
    void submitInputForRoom(admittedRoom, input, sequence)
      .then(applyPhysicsSnapshot)
      .catch(() => {
        // Hosting may not have been started yet; presence/chat remain available.
      });
  }, [admittedRoom, applyPhysicsSnapshot, domeRecovery.state, submitInputForRoom]);

  useEffect(() => {
    if (!admittedRoom?.metaverse || domeRecovery.state !== 'online') return;
    const keepAlive = () => {
      void submitInputForRoom(admittedRoom, { type: 'keep_alive' })
        .then(applyPhysicsSnapshot)
        .catch((error: unknown) => {
          const message = error instanceof Error ? error.message : String(error);
          if (message.includes('BLOCKED') || message.includes('VISITOR_BLOCKED')) {
            setPendingEvacuationReason('blocked');
          } else if (message.includes('ACCESS_DENIED') || message.includes('ACCESS_REVOKED')) {
            setPendingEvacuationReason('access_revoked');
          }
        });
    };
    const intervalId = window.setInterval(keepAlive, 5_000);
    return () => window.clearInterval(intervalId);
  }, [admittedRoom, applyPhysicsSnapshot, domeRecovery.state, submitInputForRoom]);

  useEffect(() => {
    if (!admittedRoom || domeRecovery.state !== 'online') {
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
      void actions.publishRoomEvent(admittedRoom.room_id, localPeerId, now, {
        type: 'presence_join',
        presence: {
          room_id: admittedRoom.room_id,
          peer_id: localPeerId,
          display_name: localDisplayName,
          avatar_asset_ref: localAvatarAssetRef,
          joined_at: joinedAt,
          last_seen_at: now,
        },
      }).catch(() => {
        // Browser-only fallback is handled by the local scene.
      });
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
    admittedRoom,
    domeRecovery.state,
    submitAuthoritativeInput,
  ]);

  useEffect(() => {
    if (domeRecovery.state === 'online') return;
    disableMicrophone();
    stopSpatialAudio();
  }, [disableMicrophone, domeRecovery.state, stopSpatialAudio]);

  useEffect(() => stopSpatialAudio(), [admittedRoom?.room_id, stopSpatialAudio, transitionNeighbors]);

  useEffect(() => {
    if (!admittedRoom || (roomConnectionState !== 'stale' && roomConnectionState !== 'offline')) {
      return;
    }
    const now = Date.now();
    if (now - lastRecoveryAtRef.current < METAVERSE_ROOM_RECOVERY_MS) {
      return;
    }
    lastRecoveryAtRef.current = now;
    resetBackendEventCursor();
    lastPhysicsSnapshotSequenceRef.current = 0;
    if (roomConnectionState === 'stale') {
      setRecoveringUntil(now + 3_000);
    }
    void Promise.resolve(actions.refresh()).catch(() => {
      setPollErrorCount((current) => current + 1);
    });
  }, [actions, admittedRoom, resetBackendEventCursor, roomConnectionState, setPollErrorCount]);

  function resetRoomRuntimeState() {
    setRemoteTransforms({});
    setPeerPresence({});
    setLatestChatByPeer({});
    setPollErrorCount(0);
    setLastRoomActivityAt(Date.now());
    setRecoveringUntil(0);
    setLastSentSeq(0);
    setDomeRecovery(ONLINE_DOME_RECOVERY);
    lastSentTransformRef.current = null;
    resetBackendEventCursor();
  }

  function setTransitionPreparing(direction: DomeDirection, preparing: boolean) {
    setTransitionPreparingDirections((current) => {
      const next = new Set(current);
      if (preparing) next.add(direction);
      else next.delete(direction);
      return next;
    });
  }

  const abortTransitionAttempt = useCallback(async (attempt: TransitionAttempt) => {
    if (attempt.phase === 'target_committed') return;
    attempt.cancelled = true;
    if (attempt.ticket) {
      await actions.abortTransition(attempt.ticket).catch(() => undefined);
    }
    await submitInputForRoom(attempt.sourceRoom, {
      type: 'abort_transition',
      transition_id: attempt.id,
    }).catch(() => undefined);
    if (transitionAttemptRef.current === attempt) {
      transitionAttemptRef.current = null;
    }
    setTransitionPreparingDirections((current) => {
      const next = new Set(current);
      next.delete(attempt.neighbor.direction);
      return next;
    });
  }, [actions, submitInputForRoom]);

  function beginTransitionAttempt(neighbor: DomeNeighborTransitionView) {
    if (!admittedRoom?.metaverse || transitionAttemptRef.current) return;
    const transitionId = `dome-transition-${globalThis.crypto?.randomUUID?.() ?? Date.now()}`;
    const attempt: TransitionAttempt = {
      id: transitionId,
      sourceRoom: admittedRoom,
      neighbor,
      phase: 'preparing',
      ticket: null,
      cancelled: false,
    };
    transitionAttemptRef.current = attempt;
    setTransitionPreparing(neighbor.direction, true);
    void (async () => {
      try {
        await submitInputForRoom(attempt.sourceRoom, {
          type: 'prepare_transition',
          transition_id: attempt.id,
          direction: neighbor.direction,
        });
        if (attempt.cancelled) return;
        attempt.ticket = await actions.prepareTransition({
          transition_id: attempt.id,
          connection_id: neighbor.connectionId,
          topology_digest: neighbor.topologyDigest,
          spatial_context: attempt.sourceRoom.metaverse!.spatial_context,
          source_instance_id: attempt.sourceRoom.metaverse!.instance_id,
          source_instance_generation: attempt.sourceRoom.metaverse!.instance_generation,
          target_instance_id: neighbor.room.metaverse!.instance_id,
          target_instance_generation: neighbor.room.metaverse!.instance_generation,
          participant_pubkey: syncStatus.local_author_pubkey,
          direction: neighbor.direction,
          requested_at: Date.now(),
        });
        if (attempt.cancelled) {
          await abortTransitionAttempt(attempt);
          return;
        }
        attempt.phase = 'provisional';
        setTransitionPreparing(neighbor.direction, false);
      } catch (transitionError) {
        await abortTransitionAttempt(attempt);
        setTransitionNeighbors((current) => current.map((candidate) =>
          candidate.connectionId === neighbor.connectionId
            ? { ...candidate, boundaryState: 'error' }
            : candidate
        ));
        onError(
          transitionError instanceof Error
            ? transitionError.message
            : 'Dome transition preparation failed'
        );
      }
    })();
  }

  function commitTransitionAttempt(attempt: TransitionAttempt, transform: AvatarTransform) {
    if (attempt.phase !== 'provisional' || attempt.cancelled || !attempt.ticket) return;
    attempt.phase = 'committing';
    const targetPosition = transformAvatarBetweenDomes(
      transform.position,
      attempt.neighbor.relativeCoordinateCm
    );
    const targetTransform: AvatarTransform = {
      ...transform,
      roomId: attempt.neighbor.room.room_id,
      seq: 0,
      position: targetPosition,
      sentAt: Date.now(),
    };
    void (async () => {
      try {
        await actions.commitTransition(attempt.ticket!, targetPosition, transform.rotation);
        attempt.phase = 'target_committed';
        transitionAttemptRef.current = null;
        setTransitionPreparing(attempt.neighbor.direction, false);
        setHandoffTransform(targetTransform);
        lastSentTransformRef.current = targetTransform;
        setLastSentSeq(0);
        setJoinedRoomIds((current) => {
          const next = new Set(current);
          next.delete(attempt.sourceRoom.room_id);
          next.add(attempt.neighbor.room.room_id);
          return next;
        });
        setSelectedRoomId(attempt.neighbor.room.room_id);
        if (attempt.neighbor.room.metaverse) {
          writeLastVisitedDome(
            syncStatus.local_author_pubkey,
            attempt.neighbor.room.metaverse.spatial_context,
            attempt.neighbor.room.metaverse.instance_id
          );
        }
        onError(null);
        let sourceCompleted = false;
        for (const retryDelay of [0, 250, 1_000]) {
          if (retryDelay > 0) {
            await new Promise((resolve) => window.setTimeout(resolve, retryDelay));
          }
          try {
            await submitInputForRoom(attempt.sourceRoom, {
              type: 'complete_transition',
              transition_id: attempt.id,
            });
            sourceCompleted = true;
            break;
          } catch {
            // The destination remains authoritative; retry only source cleanup.
          }
        }
        if (sourceCompleted) {
          const leftAt = Date.now();
          await actions.publishRoomEvent(attempt.sourceRoom.room_id, localPeerId, leftAt, {
            type: 'presence_leave',
            room_id: attempt.sourceRoom.room_id,
            peer_id: localPeerId,
            left_at: leftAt,
          }).catch(() => undefined);
        } else {
          onError('Destination committed; source Dome cleanup will require resynchronization');
        }
      } catch (transitionError) {
        await abortTransitionAttempt(attempt);
        setTransitionNeighbors((current) => current.map((candidate) =>
          candidate.connectionId === attempt.neighbor.connectionId
            ? { ...candidate, boundaryState: 'error' }
            : candidate
        ));
        onError(
          transitionError instanceof Error
            ? transitionError.message
            : 'Dome transition commit failed'
        );
      }
    })();
  }

  const joinRoom = useCallback(async (
    roomId: string,
    reportError = true,
    preserveCurrent = false
  ): Promise<boolean> => {
    const room = rooms.find((candidate) => candidate.room_id === roomId);
    if (!room?.metaverse) return false;
    const attempt = transitionAttemptRef.current;
    if (attempt) void abortTransitionAttempt(attempt);
    if (!preserveCurrent) {
      setHandoffTransform(null);
      setSelectedRoomId(roomId);
      setAdmissionConfirmed(false);
    }
    setAdmissionStatus('admitting');
    try {
      const avatarCollider = await resolveLocalAvatarCollider();
      const snapshot = await submitInputForRoom(room, {
        type: 'join',
        avatar_collider: avatarCollider,
      });
      const body = snapshot.bodies.find(
        (candidate) => candidate.entity_id === `avatar:${syncStatus.local_author_pubkey}`
      );
      if (!body) throw new Error('DOME_ENTRY_CONFIRMATION_MISSING_AVATAR');
      const initialTransform: AvatarTransform = {
        roomId: room.room_id,
        peerId: localPeerId,
        seq: snapshot.sequence,
        position: body.position,
        rotation: body.rotation,
        animation: normalizeAvatarAnimationState(body.animation),
        sentAt: snapshot.simulated_at,
      };
      lastSentTransformRef.current = initialTransform;
      setHandoffTransform(initialTransform);
      applyPhysicsSnapshot(snapshot);
      setJoinedRoomIds((current) => new Set(current).add(roomId));
      setAdmissionConfirmed(true);
      setSelectedRoomId(roomId);
      setAdmissionStatus('joined');
      writeLastVisitedDome(
        syncStatus.local_author_pubkey,
        room.metaverse.spatial_context,
        room.metaverse.instance_id
      );
      onError(null);
      return true;
    } catch (entryError) {
      if (!preserveCurrent) {
        setAdmissionConfirmed(false);
        setAdmissionStatus('selection');
      } else {
        setAdmissionStatus('joined');
      }
      if (reportError) {
        onError(entryError instanceof Error ? entryError.message : 'Dome entry failed');
      }
      return false;
    }
  }, [
    abortTransitionAttempt,
    applyPhysicsSnapshot,
    localPeerId,
    onError,
    resolveLocalAvatarCollider,
    rooms,
    submitInputForRoom,
    syncStatus.local_author_pubkey,
  ]);

  const evacuate = useCallback(async (
    reason: 'host_offline' | 'access_revoked' | 'blocked' | 'user_requested'
  ): Promise<boolean> => {
    if (!admittedRoom || evacuationRunningRef.current) return false;
    evacuationRunningRef.current = true;
    const sourceRoom = admittedRoom;
    setDomeRecovery({ state: 'evacuating', secondsRemaining: null, reason, targetTitle: null });
    const adjacent = transitionNeighbors
      .filter((neighbor) => neighbor.boundaryState === 'ready')
      .map((neighbor) => neighbor.room);
    const ordered = reason === 'user_requested'
      ? [...entryCandidates, ...adjacent]
      : [...adjacent, ...entryCandidates];
    const seen = new Set<string>([sourceRoom.room_id]);
    const candidates = ordered.filter((candidate) => {
      if (seen.has(candidate.room_id)) return false;
      seen.add(candidate.room_id);
      return true;
    });
    try {
      for (const candidate of candidates) {
        setDomeRecovery({
          state: 'evacuating',
          secondsRemaining: null,
          reason,
          targetTitle: candidate.title,
        });
        if (!await joinRoom(candidate.room_id, false, true)) continue;
        await submitInputForRoom(sourceRoom, { type: 'leave' }).catch(() => undefined);
        setJoinedRoomIds((current) => {
          const next = new Set(current);
          next.delete(sourceRoom.room_id);
          next.add(candidate.room_id);
          return next;
        });
        setDomeRecovery({ state: 'online', secondsRemaining: null, reason: null, targetTitle: null });
        onError(t('recovery.moved', { target: candidate.title }));
        return true;
      }
      setSelectedRoomId(null);
      setAdmissionConfirmed(false);
      setAdmissionStatus('selection');
      entryAutoDisabledRef.current = true;
      disableMicrophone();
      stopSpatialAudio();
      setDomeRecovery({ state: 'no_candidate', secondsRemaining: null, reason, targetTitle: null });
      onError(t('recovery.noCandidate'));
      return false;
    } finally {
      evacuationRunningRef.current = false;
    }
  }, [
    admittedRoom,
    disableMicrophone,
    entryCandidates,
    joinRoom,
    onError,
    stopSpatialAudio,
    submitInputForRoom,
    t,
    transitionNeighbors,
  ]);

  useEffect(() => {
    if (!pendingEvacuationReason) return;
    const reason = pendingEvacuationReason;
    setPendingEvacuationReason(null);
    setDomeRecovery({ state: 'closed', secondsRemaining: 0, reason, targetTitle: null });
    void evacuate(reason);
  }, [evacuate, pendingEvacuationReason]);

  useEffect(() => {
    if (!admittedRoom?.metaverse) return;
    let cancelled = false;
    const pollHosting = async () => {
      try {
        const hosting = await actions.getHosting(
          admittedRoom.metaverse!.spatial_context,
          admittedRoom.metaverse!.instance_id
        );
        if (cancelled) return;
        if (hosting.state.kind === 'grace_period') {
          const deadline = (hosting.state.last_heartbeat_at ?? Date.now()) + 15_000;
          setDomeRecovery({
            state: 'offline',
            secondsRemaining: Math.max(0, Math.ceil((deadline - Date.now()) / 1_000)),
            reason: 'host_offline',
            targetTitle: null,
          });
        } else if (hosting.state.kind === 'closed' && hosting.state.reason === 'heartbeat_timeout') {
          setDomeRecovery({ state: 'closed', secondsRemaining: 0, reason: 'host_offline', targetTitle: null });
          void evacuate('host_offline');
        } else if (hosting.state.kind === 'owner_hosted' || hosting.state.kind === 'community_node_hosted') {
          setDomeRecovery({ state: 'online', secondsRemaining: null, reason: null, targetTitle: null });
        }
      } catch {
        setDomeRecovery((current) => current.state === 'online' ? {
          state: 'offline',
          secondsRemaining: 15,
          reason: 'host_offline',
          targetTitle: null,
        } : current);
      }
    };
    void pollHosting();
    const intervalId = window.setInterval(() => void pollHosting(), 1_000);
    return () => {
      cancelled = true;
      window.clearInterval(intervalId);
    };
  }, [actions, admittedRoom, evacuate]);

  useEffect(() => {
    const contextKey = spatialContextKey(entryContext);
    if (entryContextKeyRef.current !== contextKey) {
      entryContextKeyRef.current = contextKey;
      entryAttemptKeyRef.current = null;
      entryAutoDisabledRef.current = false;
      setAdmissionConfirmed(false);
      setSelectedRoomId(null);
    }
    if (admissionConfirmed || entryAutoDisabledRef.current) return;
    const attemptKey = `${contextKey}:${entryCandidates.map((room) => room.room_id).join(',')}`;
    if (entryAttemptKeyRef.current === attemptKey) return;
    entryAttemptKeyRef.current = attemptKey;
    if (entryCandidates.length === 0) {
      setAdmissionStatus('selection');
      return;
    }
    setAdmissionStatus('resolving');
    let cancelled = false;
    void (async () => {
      for (const candidate of entryCandidates) {
        if (cancelled) return;
        if (await joinRoom(candidate.room_id, false)) return;
      }
      if (!cancelled) {
        setAdmissionStatus('selection');
        onError(t('entry.noAvailable'));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [admissionConfirmed, entryCandidates, entryContext, joinRoom, onError, t]);

  function selectCreatedRoom(roomId: string) {
    pendingCreatedRoomIdRef.current = roomId;
    setSelectedRoomId(roomId);
    setAdmissionConfirmed(false);
    setAdmissionStatus('selection');
    if (rooms.some((room) => room.room_id === roomId)) {
      void joinRoom(roomId);
    }
  }

  function leaveRoom() {
    if (!admittedRoom) {
      setSelectedRoomId(null);
      return;
    }
    const attempt = transitionAttemptRef.current;
    if (attempt) void abortTransitionAttempt(attempt);
    const roomId = admittedRoom.room_id;
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
    setAdmissionConfirmed(false);
    setAdmissionStatus('selection');
    entryAutoDisabledRef.current = true;
    disableMicrophone();
    stopSpatialAudio();
    resetRoomRuntimeState();
  }

  function handleLocalTransform(transform: AvatarTransform) {
    if (domeRecovery.state !== 'online') return;
    const previous = lastSentTransformRef.current;
    lastSentTransformRef.current = transform;
    setLastSentSeq(transform.seq);
    submitAuthoritativeInput({
      type: 'move',
      position: transform.position,
      rotation: transform.rotation,
      animation: transform.animation,
    }, transform.seq);
    const attempt = transitionAttemptRef.current;
    const inZone = transitionNeighborInZone(transform.position, transitionNeighbors);
    if (!attempt && inZone) {
      beginTransitionAttempt(inZone);
      return;
    }
    if (!attempt) return;
    if (
      attempt.phase !== 'committing' &&
      domeTransitionProgress(transform.position, attempt.neighbor.direction) <= 0
    ) {
      void abortTransitionAttempt(attempt);
      return;
    }
    const crossed = transitionNeighborAtPosition(previous?.position ?? null, transform.position, [
      attempt.neighbor,
    ]);
    if (crossed && attempt.phase === 'provisional') {
      commitTransitionAttempt(attempt, transform);
    }
  }

  function handleSendMessage(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!admittedRoom || domeRecovery.state !== 'online' || !messageDraft.trim()) {
      return;
    }
    const message: RoomChatMessage = {
      roomId: admittedRoom.room_id,
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
      .publishRoomEvent(admittedRoom.room_id, localPeerId, Date.now(), {
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
    if (!admittedRoom) {
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
    if (!admittedRoom?.metaverse) return;
    const prop = admittedRoom.metaverse.dome.customization.persistent_props.find(
      (candidate) => candidate.prop_id === sharedObjectRef.current.object_id
    );
    if (!prop || prop.visual_only || !prop.interactions.includes(interaction)) return;

    const input = createDomeInteractionInput(interaction, prop.prop_id, localPeerId);
    const current = sharedObjectRef.current;
    if (input.type === 'sit') {
      const previous = lastSentTransformRef.current;
      handleLocalTransform({
        roomId: admittedRoom.room_id,
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
    if (!admittedRoom?.metaverse) {
      throw new Error(t('errors.roomRequired'));
    }
    const snapshot = await actions.submitSessionInput(
      admittedRoom.metaverse.spatial_context,
      admittedRoom.metaverse.instance_id,
      Date.now(),
      input
    );
    applyPhysicsSnapshot(snapshot);
  }, [actions, admittedRoom, applyPhysicsSnapshot, t]);

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
    admittedRoom,
    admissionStatus,
    localPeerId,
    remoteTransforms,
    peerPresence,
    messages,
    latestChatByPeer,
    messageDraft,
    setMessageDraft,
    sharedObject,
    sessionProps,
    transitionNeighbors,
    transitionBoundaryStates,
    handoffTransform,
    lastSentSeq,
    lastReceivedAt,
    remoteAnimationSummary,
    roomConnectionState,
    knownPeerCount,
    clockNow,
    domeRecovery,
    returnHome: () => void evacuate('user_requested'),
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
    microphoneEnabled,
    toggleMicrophone,
  };
}
