import {
  type DesktopApi,
  type GameScoreView,
  type MetaverseAssetRef,
  type MetaverseRoomEventV1,
  type MetaverseRoomEventView,
  type TimelineScope,
} from '@/lib/api';
import { createDefaultMetaverseRoomState } from '@/components/extended/metaverse/DomeSceneModel';

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
  | 'moveDome'
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
          phase_label: 'fixed-dome-v1',
          scores: [],
          room_kind: 'metaverse_room',
          metaverse: createDefaultMetaverseRoomState(maxPeers, {
            roomId,
            topicId: topic,
            channelId,
            ownerPubkey: syncStatus.local_author_pubkey,
          }),
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
      customization
    ) {
      const now = Date.now();
      gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) =>
        room.room_id === roomId && room.metaverse
          ? withGameRoomDefaults({
              ...room,
              status,
              metaverse: {
                ...room.metaverse,
                dome: { ...room.metaverse.dome, customization },
              },
              updated_at: now,
              manifest_blob_hash: `mock-${roomId}-${now}`,
            })
          : room
      );
    },
    async moveDome(sourceTopic, moveId, sourceInstanceId, targetContext) {
      const source = (gameRoomsByTopic[sourceTopic] ?? []).find(
        (room) => room.room_id === sourceInstanceId && room.metaverse
      );
      if (!source?.metaverse) throw new Error('source Dome instance not found');
      const targetTopic = targetContext.topic_id;
      const targetRoomId = `moved-${sourceInstanceId}`;
      gameRoomsByTopic[sourceTopic] = (gameRoomsByTopic[sourceTopic] ?? []).filter(
        (room) => room.room_id !== sourceInstanceId
      );
      gameRoomsByTopic[targetTopic] = [
        withGameRoomDefaults({
          ...source,
          room_id: targetRoomId,
          channel_id: targetContext.kind === 'channel' ? targetContext.channel_id : null,
          metaverse: {
            ...source.metaverse,
            instance_id: targetRoomId,
            spatial_context: targetContext,
            session_id: targetRoomId,
            relationship_detach: null,
            replacement_instance_id: null,
          },
        }),
        ...(gameRoomsByTopic[targetTopic] ?? []),
      ];
      return {
        move_id: moveId,
        owner_pubkey: syncStatus.local_author_pubkey,
        source_instance_id: sourceInstanceId,
        source_context: source.metaverse.spatial_context,
        source_generation: source.metaverse.instance_generation,
        target_instance_id: targetRoomId,
        target_context: targetContext,
        target_generation: 1,
        preset_ref: source.metaverse.preset_ref,
        phase: 'completed',
        failure_reason: null,
        updated_at: Date.now(),
      };
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
          spatial_context: (gameRoomsByTopic[topic] ?? []).find((room) => room.room_id === roomId)?.metaverse?.spatial_context ?? { kind: 'topic', topic_id: topic },
          instance_generation: (gameRoomsByTopic[topic] ?? []).find((room) => room.room_id === roomId)?.metaverse?.instance_generation ?? 1,
          session_id: roomId,
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
