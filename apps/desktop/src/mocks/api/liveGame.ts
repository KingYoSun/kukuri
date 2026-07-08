import {
  type DesktopApi,
  type GameScoreView,
  type MetaverseAssetRef,
  type MetaverseRoomEventV1,
  type MetaverseRoomEventView,
  type TimelineScope,
} from '@/lib/api';

import {
  filterChannelScopedItems,
  withGameRoomDefaults,
  withLiveSessionDefaults,
} from '../desktopMockModel';
import { type MockRuntime } from '../mockRuntime';

type LiveGameMock = Pick<
  DesktopApi,
  | 'listLiveSessions'
  | 'createLiveSession'
  | 'endLiveSession'
  | 'joinLiveSession'
  | 'leaveLiveSession'
  | 'listGameRooms'
  | 'createGameRoom'
  | 'createMetaverseRoom'
  | 'updateGameRoom'
  | 'updateMetaverseRoom'
  | 'publishMetaverseRoomEvent'
  | 'listMetaverseRoomEvents'
  | 'importMetaverseRoomAsset'
>;

export function createLiveGameMock(runtime: MockRuntime): LiveGameMock {
  const {
    liveSessionsByTopic,
    gameRoomsByTopic,
    joinedChannelsByTopic,
    syncStatus,
    metaverseRoomEventsByRoom,
    metaverseAssetPayloads,
    mutedAuthorPubkeys,
  } = runtime;

  return {
    async listLiveSessions(topic, scope: TimelineScope = { kind: 'public' }) {
      const muted = mutedAuthorPubkeys();
      return filterChannelScopedItems(
        liveSessionsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      ).filter((session) => !muted.has(session.host_pubkey));
    },
    async createLiveSession(topic, title, description, channelRef = { kind: 'public' }) {
      runtime.sequence += 1;
      const sessionId = `live-${runtime.sequence}`;
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      liveSessionsByTopic[topic] = [
        withLiveSessionDefaults({
          session_id: sessionId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Live',
          started_at: Date.now(),
          ended_at: null,
          viewer_count: 0,
          joined_by_me: false,
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...(liveSessionsByTopic[topic] ?? []),
      ];
      return sessionId;
    },
    async endLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? { ...session, status: 'Ended', ended_at: Date.now(), joined_by_me: false }
          : session
      );
    },
    async joinLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? { ...session, joined_by_me: true, viewer_count: session.viewer_count + 1 }
          : session
      );
    },
    async leaveLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? {
              ...session,
              joined_by_me: false,
              viewer_count: Math.max(0, session.viewer_count - 1),
            }
          : session
      );
    },
    async listGameRooms(topic, scope: TimelineScope = { kind: 'public' }) {
      const muted = mutedAuthorPubkeys();
      return filterChannelScopedItems(
        gameRoomsByTopic[topic] ?? [],
        scope,
        joinedChannelsByTopic[topic] ?? []
      ).filter((room) => !muted.has(room.host_pubkey));
    },
    async createGameRoom(topic, title, description, participants, channelRef = { kind: 'public' }) {
      runtime.sequence += 1;
      const roomId = `game-${runtime.sequence}`;
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      const scores: GameScoreView[] = participants.map((label, index) => ({
        participant_id: `participant-${index + 1}`,
        label,
        score: 0,
      }));
      gameRoomsByTopic[topic] = [
        withGameRoomDefaults({
          room_id: roomId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Waiting',
          phase_label: null,
          scores,
          updated_at: Date.now(),
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...(gameRoomsByTopic[topic] ?? []),
      ];
      return roomId;
    },
    async createMetaverseRoom(
      topic,
      title,
      description,
      maxPeers = null,
      channelRef = { kind: 'public' }
    ) {
      runtime.sequence += 1;
      const roomId = `meta-${runtime.sequence}`;
      const channelId = channelRef.kind === 'private_channel' ? channelRef.channel_id : null;
      const now = Date.now();
      gameRoomsByTopic[topic] = [
        withGameRoomDefaults({
          room_id: roomId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Waiting',
          phase_label: 'metaverse-mvp',
          scores: [],
          room_kind: 'metaverse_room',
          metaverse: {
            world_version: 1,
            max_peers: maxPeers,
            scene: {
              ground: 'default',
              shared_object: {
                object_id: 'mvp-object-1',
                asset_ref: null,
                primitive_fallback: 'cube',
                position: [0, 50, -240],
                rotation: [0, 0, 0],
                scale: [100, 100, 100],
                updated_by: syncStatus.local_author_pubkey,
                updated_at: now,
              },
            },
            default_spawn: {
              position: [0, 0, 260],
              rotation: [0, 180, 0],
            },
            asset_refs: [],
            chat_history: [],
          },
          manifest_blob_hash: `mock-${roomId}`,
          updated_at: now,
          channel_id: channelId,
          audience_label: channelId ? 'Private channel' : 'Public',
        }),
        ...(gameRoomsByTopic[topic] ?? []),
      ];
      return roomId;
    },
    async updateGameRoom(topic, roomId, status, phaseLabel, scores) {
      gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) =>
        room.room_id === roomId
          ? {
              ...room,
              status,
              phase_label: phaseLabel,
              scores: scores.map((score) => ({ ...score })),
              updated_at: Date.now(),
            }
          : room
      );
    },
    async updateMetaverseRoom(
      topic,
      roomId,
      status,
      sharedObjectPosition,
      sharedObjectRotation,
      sharedObjectScale
    ) {
      const now = Date.now();
      gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) =>
        room.room_id === roomId && room.metaverse
          ? withGameRoomDefaults({
              ...room,
              status,
              metaverse: {
                ...room.metaverse,
                scene: {
                  ...room.metaverse.scene,
                  shared_object: {
                    ...room.metaverse.scene.shared_object,
                    position: sharedObjectPosition,
                    rotation: sharedObjectRotation,
                    scale: sharedObjectScale,
                    updated_by: syncStatus.local_author_pubkey,
                    updated_at: now,
                  },
                },
              },
              updated_at: now,
              manifest_blob_hash: `mock-${roomId}-${now}`,
            })
          : room
      );
    },
    async publishMetaverseRoomEvent(topic, roomId, peerId, seq, event) {
      const now = Date.now();
      const envelopeId = `mock-metaverse-event-${now}-${seq}`;
      const view: MetaverseRoomEventView = {
        envelope_id: envelopeId,
        content: {
          event_id: envelopeId,
          topic_id: topic,
          channel_id: null,
          room_id: roomId,
          peer_id: peerId,
          seq,
          sent_at: now,
          event: event as MetaverseRoomEventV1,
        },
        envelope: {
          id: envelopeId,
          kind: 'metaverse-room-event',
          pubkey: syncStatus.local_author_pubkey,
        },
        received_at: now,
        source_peer: 'mock-local',
      };
      const key = `${topic}::${roomId}`;
      metaverseRoomEventsByRoom[key] = [...(metaverseRoomEventsByRoom[key] ?? []), view].slice(-512);
      if (event.type === 'chat_message') {
        gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) => {
          if (room.room_id !== roomId || !room.metaverse) {
            return room;
          }
          const chatHistory = [
            ...(room.metaverse.chat_history ?? []).filter(
              (message) => message.message_id !== event.message.message_id
            ),
            event.message,
          ].slice(-100);
          return withGameRoomDefaults({
            ...room,
            metaverse: {
              ...room.metaverse,
              chat_history: chatHistory,
            },
            updated_at: now,
            manifest_blob_hash: `mock-${roomId}-${now}`,
          });
        });
      }
      return view;
    },
    async listMetaverseRoomEvents(topic, roomId, afterEnvelopeId = null, limit = null) {
      const key = `${topic}::${roomId}`;
      const events = metaverseRoomEventsByRoom[key] ?? [];
      const start = afterEnvelopeId
        ? events.findIndex((event) => event.envelope_id === afterEnvelopeId) + 1
        : 0;
      const page = events.slice(Math.max(0, start));
      return typeof limit === 'number' && page.length > limit ? page.slice(page.length - limit) : page;
    },
    async importMetaverseRoomAsset(_topic, roomId, kind, mimeType, name, dataBase64) {
      const hash = `mock-metaverse-asset-${roomId}-${Object.keys(metaverseAssetPayloads).length + 1}`;
      metaverseAssetPayloads[hash] = {
        bytes_base64: dataBase64,
        mime: mimeType,
      };
      return {
        kind,
        blob_hash: hash,
        mime_type: mimeType,
        size_bytes: Math.ceil((dataBase64.length * 3) / 4),
        name,
      } satisfies MetaverseAssetRef;
    },
  };
}
