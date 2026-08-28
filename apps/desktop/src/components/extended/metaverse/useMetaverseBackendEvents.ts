import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from 'react';

import type { GameRoomView, MetaverseRoomEventView } from '@/lib/api';
import type {
  AvatarTransform,
  LatestChatBubble,
  PeerPresence,
  RoomChatMessage,
} from '../MetaverseSceneModel';
import { mergeRoomChatMessages } from '../MetaverseSceneModel';
import type { DomeNeighborTransitionView } from './DomeTransitionModel';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import { chatMessageFromApi, latestChatBubbleFromMessage } from './MetaverseRoomSessionSupport';

type UseMetaverseBackendEventsArgs = {
  actions: MetaverseRoomActions;
  selectedRoom: GameRoomView | null;
  transitionNeighbors: DomeNeighborTransitionView[];
  localPeerId: string;
  playSpatialAudioFrame: (view: MetaverseRoomEventView) => void;
  setRemoteTransforms: Dispatch<SetStateAction<Record<string, AvatarTransform>>>;
};

export function useMetaverseBackendEvents({
  actions,
  selectedRoom,
  transitionNeighbors,
  localPeerId,
  playSpatialAudioFrame,
  setRemoteTransforms,
}: UseMetaverseBackendEventsArgs) {
  const [peerPresence, setPeerPresence] = useState<Record<string, PeerPresence>>({});
  const [messages, setMessages] = useState<RoomChatMessage[]>([]);
  const [latestChatByPeer, setLatestChatByPeer] = useState<Record<string, LatestChatBubble>>({});
  const [pollErrorCount, setPollErrorCount] = useState(0);
  const [lastRoomActivityAt, setLastRoomActivityAt] = useState(() => Date.now());
  const cursorsRef = useRef(new Map<string, string>());

  const resetBackendEventCursor = useCallback(() => cursorsRef.current.clear(), []);

  useEffect(() => {
    if (!selectedRoom) return;
    let cancelled = false;
    let timeoutId = 0;
    const applyEvent = (view: MetaverseRoomEventView) => {
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
          [presence.peerId]: { ...current[presence.peerId], ...presence },
        }));
        if (presence.avatarAssetRef) {
          void actions.getBlobPreviewUrl(
            presence.avatarAssetRef.blob_hash,
            presence.avatarAssetRef.mime_type ?? 'model/vrm',
            presence.avatarAssetRef.kind
          ).then((avatarAssetUrl) => {
            if (!cancelled && avatarAssetUrl) {
              setPeerPresence((current) => ({
                ...current,
                [presence.peerId]: { ...current[presence.peerId], avatarAssetUrl },
              }));
            }
          }).catch(() => undefined);
        }
      } else if (event.type === 'presence_leave' && event.peer_id !== localPeerId) {
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
      } else if (event.type === 'chat_message') {
        const message = chatMessageFromApi(event.message);
        setMessages((current) => mergeRoomChatMessages(current, [message]));
        setLatestChatByPeer((current) => ({
          ...current,
          [message.authorPeerId]: latestChatBubbleFromMessage(message),
        }));
      } else if (event.type === 'spatial_audio_frame') {
        playSpatialAudioFrame(view);
      }
    };
    const poll = async () => {
      try {
        const rooms = [
          selectedRoom,
          ...transitionNeighbors
            .filter((neighbor) => neighbor.boundaryState === 'ready')
            .map((neighbor) => neighbor.room),
        ].slice(0, 5);
        const batches = await Promise.all(rooms.map(async (room) => ({
          roomId: room.room_id,
          events: await actions.listRoomEvents(
            room.room_id,
            cursorsRef.current.get(room.room_id) ?? null,
            64
          ),
        })));
        if (!cancelled) {
          for (const batch of batches) {
            batch.events.forEach(applyEvent);
            const last = batch.events.at(-1);
            if (last) cursorsRef.current.set(batch.roomId, last.envelope_id);
          }
          setPollErrorCount(0);
        }
      } catch {
        if (!cancelled) setPollErrorCount((current) => current + 1);
      } finally {
        if (!cancelled) timeoutId = window.setTimeout(() => void poll(), 180);
      }
    };
    void poll();
    return () => {
      cancelled = true;
      window.clearTimeout(timeoutId);
    };
  }, [actions, localPeerId, playSpatialAudioFrame, selectedRoom, setRemoteTransforms, transitionNeighbors]);

  useEffect(() => {
    const liveRoomIds = new Set([
      selectedRoom?.room_id,
      ...transitionNeighbors
        .filter((neighbor) => neighbor.boundaryState === 'ready')
        .map((neighbor) => neighbor.room.room_id),
    ].filter((roomId): roomId is string => Boolean(roomId)));
    for (const roomId of cursorsRef.current.keys()) {
      if (!liveRoomIds.has(roomId)) cursorsRef.current.delete(roomId);
    }
  }, [selectedRoom?.room_id, transitionNeighbors]);

  useEffect(() => {
    const intervalId = window.setInterval(() => {
      const cutoff = Date.now() - 10_000;
      setPeerPresence((current) => Object.fromEntries(
        Object.entries(current).filter(([, presence]) => presence.lastSeenAt >= cutoff)
      ));
    }, 1_000);
    return () => window.clearInterval(intervalId);
  }, []);

  return {
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
  };
}
