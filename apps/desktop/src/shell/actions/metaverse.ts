import type { ChannelRef, DesktopApi } from '@/lib/api';
import type { MetaverseRoomActions } from '@/components/extended/metaverse/MetaverseRoomActions';

type CreateMetaverseRoomActionsArgs = {
  api: DesktopApi;
  activeTopic: string;
  activeComposeChannel: ChannelRef;
  onRefresh: () => Promise<void>;
};

export function createMetaverseRoomActions({
  api,
  activeTopic,
  activeComposeChannel,
  onRefresh,
}: CreateMetaverseRoomActionsArgs): MetaverseRoomActions {
  return {
    createRoom: (input) =>
      api.createMetaverseRoom(
        activeTopic,
        input.title,
        input.description,
        input.maxPeers,
        activeComposeChannel
      ),
    publishRoomEvent: (roomId, peerId, seq, event) =>
      api
        .publishMetaverseRoomEvent(activeTopic, roomId, peerId, seq, event)
        .then(() => undefined),
    listRoomEvents: (roomId, afterEnvelopeId, limit) =>
      api.listMetaverseRoomEvents(activeTopic, roomId, afterEnvelopeId, limit),
    importRoomAsset: (roomId, kind, mime, name, dataBase64) =>
      api.importMetaverseRoomAsset(
        activeTopic,
        roomId,
        kind,
        mime,
        name,
        dataBase64
      ),
    getBlobPreviewUrl: (blobHash, mime) => api.getBlobPreviewUrl(blobHash, mime),
    updateRoom: (roomId, status, position, rotation, scale) =>
      api.updateMetaverseRoom(activeTopic, roomId, status, position, rotation, scale),
    refresh: onRefresh,
  };
}
