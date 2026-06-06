import { useEffect, useId, useMemo, useRef, useState, type FormEvent } from 'react';
import {
  AlertTriangle,
  Box,
  ChevronDown,
  Cuboid,
  MessageSquare,
  Move3D,
  PanelRightClose,
  PanelRightOpen,
  Play,
  RefreshCw,
  Send,
  Wifi,
  WifiOff,
  X,
} from 'lucide-react';

import { AuthorAvatar } from '@/components/core/AuthorAvatar';
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
  AuthorSocialView,
  Profile,
  SharedRoomObjectV1,
  SyncStatus,
} from '@/lib/api';
import { formatLocalizedTime } from '@/i18n/format';
import type { SupportedLocale } from '@/i18n';
import { blobToBase64 } from '@/lib/attachments';
import { MetaverseScene } from './MetaverseScene';
import {
  DEFAULT_AVATAR_ASSET_NAME,
  DEFAULT_AVATAR_ASSET_URL,
  DEFAULT_SHARED_OBJECT,
  METAVERSE_CHAT_BUBBLE_TTL_MS,
  METAVERSE_ROOM_HEARTBEAT_MS,
  METAVERSE_ROOM_RECOVERY_MS,
  METAVERSE_ROOM_STALE_MS,
  isNewerSharedObject,
  mergeRoomChatMessages,
  normalizeAvatarAnimationState,
  type AvatarAssetStatus,
  type AvatarTransform,
  type LatestChatBubble,
  type MetaverseRoomConnectionState,
  type MetaverseRoomEvent,
  type MetaverseVec3,
  type PeerPresence,
  type RoomChatMessage,
} from './MetaverseSceneModel';

type MetaverseRoomPanelProps = {
  api: DesktopApi;
  activeTopic: string;
  activeComposeChannel: ChannelRef;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  localProfile?: Profile | null;
  knownAuthorsByPubkey?: Record<string, AuthorSocialView>;
  mediaObjectUrls?: Record<string, string | null>;
  onRefresh: () => Promise<void>;
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

function connectionStateLabel(state: MetaverseRoomConnectionState) {
  if (state === 'live') {
    return 'Live';
  }
  if (state === 'recovering') {
    return 'Recovering';
  }
  if (state === 'stale') {
    return 'Stale';
  }
  return 'Offline';
}

function connectionStateDetail(state: MetaverseRoomConnectionState) {
  if (state === 'live') {
    return 'Room events are flowing';
  }
  if (state === 'recovering') {
    return 'Refreshing room connectivity';
  }
  if (state === 'stale') {
    return 'No room activity recently';
  }
  return 'Peer connectivity is unavailable';
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

function ConnectionStateIcon({ state }: { state: MetaverseRoomConnectionState }) {
  if (state === 'live') {
    return <Wifi className='size-4' aria-hidden='true' />;
  }
  if (state === 'recovering') {
    return <RefreshCw className='size-4' aria-hidden='true' />;
  }
  if (state === 'stale') {
    return <AlertTriangle className='size-4' aria-hidden='true' />;
  }
  return <WifiOff className='size-4' aria-hidden='true' />;
}

export function MetaverseRoomPanel({
  api,
  activeTopic,
  activeComposeChannel,
  rooms,
  syncStatus,
  locale,
  localProfile = null,
  knownAuthorsByPubkey = {},
  mediaObjectUrls = {},
  onRefresh,
}: MetaverseRoomPanelProps) {
  const [createOpen, setCreateOpen] = useState(false);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [maxPeers, setMaxPeers] = useState('8');
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [selectedRoomId, setSelectedRoomId] = useState<string | null>(null);
  const [joinedRoomIds, setJoinedRoomIds] = useState<Set<string>>(() => new Set());
  const [hudOpen, setHudOpen] = useState(true);
  const [chatOpen, setChatOpen] = useState(true);
  const [hudDebugOpen, setHudDebugOpen] = useState(false);
  const [remoteTransforms, setRemoteTransforms] = useState<Record<string, AvatarTransform>>({});
  const [peerPresence, setPeerPresence] = useState<Record<string, PeerPresence>>({});
  const [messages, setMessages] = useState<RoomChatMessage[]>([]);
  const [latestChatByPeer, setLatestChatByPeer] = useState<Record<string, LatestChatBubble>>({});
  const [messageDraft, setMessageDraft] = useState('');
  const [sharedObject, setSharedObject] = useState<SharedRoomObjectV1>(DEFAULT_SHARED_OBJECT);
  const [lastSentSeq, setLastSentSeq] = useState(0);
  const [avatarAssetStatus, setAvatarAssetStatus] = useState<AvatarAssetStatus>('loading');
  const [localAvatarAssetRef, setLocalAvatarAssetRef] = useState<MetaverseAssetRef | null>(null);
  const [localAvatarAssetUrl, setLocalAvatarAssetUrl] = useState<string | null>(null);
  const [pollErrorCount, setPollErrorCount] = useState(0);
  const [lastRoomActivityAt, setLastRoomActivityAt] = useState(() => Date.now());
  const [recoveringUntil, setRecoveringUntil] = useState(0);
  const [clockNow, setClockNow] = useState(() => Date.now());
  const channelRef = useRef<BroadcastChannel | null>(null);
  const lastBackendEventEnvelopeIdRef = useRef<string | null>(null);
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
  const selectedRoomSharedObject = selectedRoom?.metaverse?.scene.shared_object ?? null;
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
  const localDisplayName = localProfile?.display_name?.trim() || localProfile?.name?.trim() || null;

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
    setRemoteTransforms({});
    setPeerPresence({});
    setMessages([]);
    setLatestChatByPeer({});
    setPollErrorCount(0);
    setLastRoomActivityAt(Date.now());
    lastBackendEventEnvelopeIdRef.current = null;
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
      if (data.type === 'avatar.transform' && data.transform.peerId !== localPeerId) {
        setRemoteTransforms((current) => ({
          ...current,
          [data.transform.peerId]: data.transform,
        }));
      }
      if (data.type === 'chat.message') {
        setMessages((current) => mergeRoomChatMessages(current, [data.message]));
        setLatestChatByPeer((current) => ({
          ...current,
          [data.message.authorPeerId]: latestChatBubbleFromMessage(data.message),
        }));
      }
      if (data.type === 'object.update' && data.object.updated_by !== localPeerId) {
        setSharedObject((current) => {
          if (!isNewerSharedObject(current, data.object)) {
            return current;
          }
          sharedObjectRef.current = data.object;
          return data.object;
        });
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
      void api.publishMetaverseRoomEvent(activeTopic, selectedRoom.room_id, localPeerId, now, {
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
    };
    publishPresence();
    const intervalId = window.setInterval(publishPresence, METAVERSE_ROOM_HEARTBEAT_MS);
    return () => {
      window.clearInterval(intervalId);
    };
  }, [
    activeTopic,
    api,
    localAvatarAssetRef,
    localAvatarAssetUrl,
    localDisplayName,
    localPeerId,
    selectedRoom,
  ]);

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
    await importAvatarBlob(await response.blob(), DEFAULT_AVATAR_ASSET_NAME);
  }

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
          void api
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
      if (event.type === 'avatar_transform' && event.transform.peer_id !== localPeerId) {
        setRemoteTransforms((current) => ({
          ...current,
          [event.transform.peer_id]: {
            roomId: event.transform.room_id,
            peerId: event.transform.peer_id,
            seq: event.transform.seq,
            position: event.transform.position,
            rotation: event.transform.rotation,
            animation: normalizeAvatarAnimationState(event.transform.animation),
            sentAt: event.transform.sent_at,
          },
        }));
      }
      if (event.type === 'chat_message') {
        const message = chatMessageFromApi(event.message);
        setMessages((current) => mergeRoomChatMessages(current, [message]));
        setLatestChatByPeer((current) => ({
          ...current,
          [message.authorPeerId]: latestChatBubbleFromMessage(message),
        }));
      }
      if (event.type === 'object_update' && event.object.updated_by !== localPeerId) {
        setSharedObject((current) => {
          if (!isNewerSharedObject(current, event.object)) {
            return current;
          }
          sharedObjectRef.current = event.object;
          return event.object;
        });
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
  }, [activeTopic, api, localPeerId, selectedRoom]);

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
    if (roomConnectionState === 'stale') {
      setRecoveringUntil(now + 3_000);
    }
    void Promise.resolve(onRefresh()).catch(() => {
      setPollErrorCount((current) => current + 1);
    });
  }, [onRefresh, roomConnectionState, selectedRoom]);

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
      pendingCreatedRoomIdRef.current = roomId;
      setJoinedRoomIds((current) => new Set(current).add(roomId));
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

  function handleJoinRoom(roomId: string) {
    setJoinedRoomIds((current) => new Set(current).add(roomId));
    setSelectedRoomId(roomId);
  }

  function hostAuthor(room: GameRoomView): Profile | AuthorSocialView | null {
    return room.host_pubkey === syncStatus.local_author_pubkey
      ? localProfile
      : knownAuthorsByPubkey[room.host_pubkey] ?? null;
  }

  function hostLabel(room: GameRoomView) {
    const host = hostAuthor(room);
    return host?.display_name?.trim() || host?.name?.trim() || room.host_pubkey.slice(0, 10);
  }

  function hostPicture(room: GameRoomView) {
    const host = hostAuthor(room);
    const pictureAssetHash = host?.picture_asset?.hash;
    if (pictureAssetHash && typeof mediaObjectUrls[pictureAssetHash] === 'string') {
      return mediaObjectUrls[pictureAssetHash];
    }
    return host?.picture ?? null;
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
          display_name: message.displayName,
          body: message.body,
          created_at: message.createdAt,
        },
      }
    ).catch(() => {
      // Browser-only fallback is handled by BroadcastChannel.
    });
  }

  function persistSharedObject(nextObject: SharedRoomObjectV1, room: GameRoomView) {
    emit({ type: 'object.update', roomId: room.room_id, object: nextObject });
    void api.publishMetaverseRoomEvent(
      activeTopic,
      room.room_id,
      localPeerId,
      Date.now(),
      {
        type: 'object_update',
        object: nextObject,
      }
    ).catch(() => {
      // Browser-only fallback is handled by BroadcastChannel.
    });
    void api.updateMetaverseRoom(
      activeTopic,
      room.room_id,
      room.status,
      nextObject.position,
      nextObject.rotation,
      nextObject.scale
    )
      .then(() => onRefresh())
      .catch((updateError) => {
        setError(updateError instanceof Error ? updateError.message : 'Failed to persist shared object');
      });
  }

  function moveSharedObject(delta: MetaverseVec3) {
    if (!selectedRoom) {
      return;
    }
    const room = selectedRoom;
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
    persistSharedObject(nextObject, room);
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
        <section className='shell-nav-accordion metaverse-create-accordion' data-open={createOpen}>
          <button
            className='shell-nav-accordion-trigger'
            type='button'
            aria-expanded={createOpen}
            onClick={() => setCreateOpen((current) => !current)}
          >
            <Cuboid className='size-4' aria-hidden='true' />
            <span className='shell-nav-accordion-title'>Create metaverse room</span>
            <ChevronDown className='shell-nav-accordion-icon size-4' aria-hidden='true' />
          </button>
          {createOpen ? (
            <form className='composer composer-compact metaverse-create-form' onSubmit={handleCreateRoom}>
              <div className='metaverse-create-form-primary'>
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
                  <span>Max peers</span>
                  <Input
                    value={maxPeers}
                    disabled={pending}
                    onChange={(event) => setMaxPeers(event.target.value)}
                  />
                </Label>
              </div>
              <Label className='metaverse-create-form-description'>
                <span>Description</span>
                <Textarea
                  value={description}
                  placeholder='Small social space'
                  disabled={pending}
                  onChange={(event) => setDescription(event.target.value)}
                />
              </Label>
              <div className='metaverse-create-form-actions'>
                <Button type='submit' disabled={pending}>
                  <Cuboid className='size-4' aria-hidden='true' />
                  Create metaverse room
                </Button>
              </div>
            </form>
          ) : null}
        </section>
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
                <div className='metaverse-room-host'>
                  <AuthorAvatar label={hostLabel(room)} picture={hostPicture(room)} size='sm' />
                  <span>Host: {hostLabel(room)}</span>
                </div>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>Updated: {formatLocalizedTime(room.updated_at, locale)}</span>
                  <span>{joinedRoomIds.has(room.room_id) ? 'Joined' : 'Not joined'}</span>
                </div>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>Manifest: {room.manifest_blob_hash ?? 'pending'}</span>
                  <span>World: {room.metaverse?.world_version ?? 1}</span>
                </div>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() => handleJoinRoom(room.room_id)}
                >
                  <Play className='size-4' aria-hidden='true' />
                  Join Room
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
              peerPresence={peerPresence}
              sharedObject={sharedObject}
              avatarAssetUrl={localAvatarAssetUrl}
              latestChatByPeer={latestChatByPeer}
              connectionState={roomConnectionState}
              now={clockNow}
              onLocalTransform={handleLocalTransform}
              onAvatarAssetStatus={setAvatarAssetStatus}
              hud={(
                <>
                  <div
                    className='metaverse-connection-badge'
                    data-state={roomConnectionState}
                    title={connectionStateDetail(roomConnectionState)}
                  >
                    <ConnectionStateIcon state={roomConnectionState} />
                    <span>{connectionStateLabel(roomConnectionState)}</span>
                  </div>
                  <div className='metaverse-hud-toolbar' data-open={hudOpen}>
                    <Button
                      variant='ghost'
                      size='icon'
                      className='metaverse-hud-icon-button'
                      type='button'
                      aria-label={hudOpen ? 'Hide room HUD' : 'Open room HUD'}
                      onClick={() => setHudOpen((open) => !open)}
                    >
                      {hudOpen ? (
                        <PanelRightClose className='size-4' aria-hidden='true' />
                      ) : (
                        <PanelRightOpen className='size-4' aria-hidden='true' />
                      )}
                    </Button>
                  </div>
                  {hudOpen ? (
                    <>
                    <aside className='metaverse-room-hud'>
                      <div className='panel-header metaverse-hud-header'>
                        <div>
                          <h3>{selectedRoom.title}</h3>
                          <small>{selectedRoom.room_id}</small>
                        </div>
                      </div>
                      <section className='metaverse-hud-accordion' data-open={hudDebugOpen}>
                        <button
                          type='button'
                          className='metaverse-hud-accordion-trigger'
                          aria-expanded={hudDebugOpen}
                          onClick={() => setHudDebugOpen((open) => !open)}
                        >
                          <span>Debug details</span>
                          <ChevronDown className='size-4' aria-hidden='true' />
                        </button>
                        {hudDebugOpen ? (
                          <div className='metaverse-room-diagnostics'>
                            <span>Topic: {activeTopic}</span>
                            <span>Local peer: {localPeerId}</span>
                            <span>Known peers: {knownPeerCount}</span>
                            <span>Last sent seq: {lastSentSeq}</span>
                            <span>Last received: {lastReceivedAt ? formatLocalizedTime(lastReceivedAt, locale) : 'none'}</span>
                            <span>Remote animation: {remoteAnimationSummary || 'none'}</span>
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
                        ) : null}
                      </section>
                      <div className='metaverse-object-controls'>
                        <strong>Avatar asset</strong>
                        <div className='metaverse-avatar-asset-controls'>
                          <Label>
                            <span className='sr-only'>VRM file</span>
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
                          <Button size='sm' variant='secondary' type='button' disabled={pending} onClick={() => void handleSampleAvatarImport()}>
                            Default
                          </Button>
                        </div>
                      </div>
                      <div className='metaverse-object-controls'>
                        <strong>
                          <Box className='size-4' aria-hidden='true' />
                          Shared object
                        </strong>
                        <div className='metaverse-nudge-grid'>
                          <Button size='sm' variant='secondary' type='button' onClick={() => void moveSharedObject([0, 0, -50])}>
                            <Move3D className='size-4' aria-hidden='true' />
                            Forward
                          </Button>
                          <Button size='sm' variant='secondary' type='button' onClick={() => void moveSharedObject([-50, 0, 0])}>Left</Button>
                          <Button size='sm' variant='secondary' type='button' onClick={() => void moveSharedObject([50, 0, 0])}>Right</Button>
                          <Button size='sm' variant='secondary' type='button' onClick={() => void moveSharedObject([0, 0, 50])}>Back</Button>
                        </div>
                      </div>
                    </aside>
                    <span className='metaverse-hud-scrollbar-indicator' aria-hidden='true'>
                      <span />
                    </span>
                    </>
                  ) : null}
                  {chatOpen ? (
                    <section className='metaverse-room-chat-log' aria-label='ROOM Chat'>
                      <div className='metaverse-room-chat-log-header'>
                        <span>
                          <MessageSquare className='size-4' aria-hidden='true' />
                          ROOM Chat
                        </span>
                        <Button
                          variant='ghost'
                          size='icon'
                          className='metaverse-chat-close-button'
                          type='button'
                          aria-label='Hide room chat'
                          onClick={() => setChatOpen(false)}
                        >
                          <X className='size-4' aria-hidden='true' />
                        </Button>
                      </div>
                      <ul className='metaverse-chat-list'>
                        {messages.map((message) => (
                          <li key={message.messageId}>
                            <strong>
                              {message.authorPeerId === localPeerId
                                ? 'You'
                                : message.displayName || message.authorPeerId.slice(0, 12)}
                              <small>{formatLocalizedTime(message.createdAt, locale)}</small>
                            </strong>
                            <span>{message.body}</span>
                          </li>
                        ))}
                      </ul>
                      <form className='metaverse-chat-form' onSubmit={handleSendMessage}>
                        <Label>
                          <span className='sr-only'>Room chat message</span>
                          <Input
                            value={messageDraft}
                            placeholder='Say something in the room'
                            onChange={(event) => setMessageDraft(event.target.value)}
                          />
                        </Label>
                        <Button size='sm' type='submit'>
                          <Send className='size-4' aria-hidden='true' />
                          Send
                        </Button>
                      </form>
                    </section>
                  ) : (
                    <Button
                      variant='secondary'
                      size='icon'
                      className='metaverse-chat-toggle'
                      type='button'
                      aria-label='Open room chat'
                      onClick={() => setChatOpen(true)}
                    >
                      <MessageSquare className='size-4' aria-hidden='true' />
                    </Button>
                  )}
                </>
              )}
            />
          </div>
        </Card>
      ) : null}
    </div>
  );
}
