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
    getBlobPreviewUrl: (blobHash, mime, metaverseKind) =>
      api.getBlobPreviewUrl(blobHash, mime, metaverseKind),
    updateRoom: (roomId, status, customization) =>
      api.updateMetaverseRoom(activeTopic, roomId, status, customization),
    getHosting: (context, instanceId) => api.getDomeHosting(context, instanceId),
    startOwnerHosting: (context, instanceId, endpointId) =>
      api.startOwnerDomeHosting(context, instanceId, endpointId, 86_400_000),
    delegateHosting: (context, instanceId, nodeId, baseUrl) =>
      api.delegateDomeHosting(context, instanceId, nodeId, baseUrl, 86_400_000),
    closeHosting: (context, instanceId) => api.closeDomeHosting(context, instanceId),
    submitSessionInput: (context, instanceId, sequence, input) =>
      api.submitDomeSessionInput(context, instanceId, sequence, input),
    commitLayout: (context, instanceId, operationId) =>
      api.commitDomeLayout(context, instanceId, operationId),
    resyncSnapshots: (context, instanceId, afterSequence) =>
      api.resyncDomeSnapshots(context, instanceId, afterSequence),
    moveRoom: (moveId, roomId, targetContext) =>
      api.moveDome(activeTopic, moveId, roomId, targetContext).then(() => undefined),
    listConnections: (context) => api.listDomeConnectionTopology(context),
    createConnectionProposal: (
      proposalId,
      context,
      proposerInstanceId,
      receiverInstanceId,
      direction
    ) =>
      api.createDomeConnectionProposal(
        proposalId,
        context,
        proposerInstanceId,
        receiverInstanceId,
        direction
      ),
    acceptConnectionProposal: (context, proposalId) =>
      api.acceptDomeConnectionProposal(context, proposalId),
    withdrawConnectionProposal: (context, proposalId) =>
      api.withdrawDomeConnectionProposal(context, proposalId),
    revokeConnection: (context, connectionId) =>
      api.revokeDomeConnection(context, connectionId),
    refresh: onRefresh,
  };
}
