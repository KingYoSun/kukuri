import { useEffect, useId, useMemo, useRef, useState, type FormEvent } from 'react';
import { Box, Cuboid, MessageSquare, Move3D, Play, Send } from 'lucide-react';

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
import { MetaverseScene } from './MetaverseScene';
import {
  DEFAULT_AVATAR_ASSET_NAME,
  DEFAULT_AVATAR_ASSET_URL,
  DEFAULT_SHARED_OBJECT,
  normalizeAvatarAnimationState,
  type AvatarAssetStatus,
  type AvatarTransform,
  type MetaverseRoomEvent,
  type MetaverseVec3,
  type RoomChatMessage,
} from './MetaverseSceneModel';

type MetaverseRoomPanelProps = {
  api: DesktopApi;
  activeTopic: string;
  activeComposeChannel: ChannelRef;
  rooms: GameRoomView[];
  syncStatus: SyncStatus;
  locale: SupportedLocale;
  onRefresh: () => Promise<void>;
};

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
  const remoteAnimationSummary = useMemo(
    () =>
      Object.values(remoteTransforms)
        .map((transform) => `${transform.peerId.slice(0, 8)}:${transform.animation}`)
        .join(', '),
    [remoteTransforms]
  );

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
