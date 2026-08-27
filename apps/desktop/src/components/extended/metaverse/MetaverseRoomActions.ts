import type {
  DomeCustomizationV1,
  DomeConnectionProposalView,
  DomeConnectionTopologyView,
  DomeConnectionView,
  DomeDirection,
  DomeHostingView,
  DomePhysicsSnapshotV1,
  DomeSessionInputKindV1,
  GameRoomView,
  MetaverseAssetRef,
  MetaverseRoomEventView,
  MetaverseRoomEventV1,
  SpatialContextV1,
} from '@/lib/api';

export type CreateMetaverseRoomActionInput = {
  title: string;
  description: string;
  maxPeers: number | null;
};

export type MetaverseRoomActions = {
  createRoom: (input: CreateMetaverseRoomActionInput) => Promise<string>;
  publishRoomEvent: (
    roomId: string,
    peerId: string,
    seq: number,
    event: MetaverseRoomEventV1
  ) => Promise<void>;
  listRoomEvents: (
    roomId: string,
    afterEnvelopeId: string | null,
    limit: number
  ) => Promise<MetaverseRoomEventView[]>;
  importRoomAsset: (
    roomId: string,
    kind: 'vrm' | 'glb' | 'texture',
    mime: string,
    name: string,
    dataBase64: string
  ) => Promise<MetaverseAssetRef>;
  getBlobPreviewUrl: (blobHash: string, mime: string) => Promise<string | null>;
  updateRoom: (
    roomId: string,
    status: GameRoomView['status'],
    customization: DomeCustomizationV1
  ) => Promise<void>;
  getHosting: (context: SpatialContextV1, instanceId: string) => Promise<DomeHostingView>;
  startOwnerHosting: (
    context: SpatialContextV1,
    instanceId: string,
    endpointId: string
  ) => Promise<DomeHostingView>;
  delegateHosting: (
    context: SpatialContextV1,
    instanceId: string,
    nodeId: string,
    baseUrl: string
  ) => Promise<DomeHostingView>;
  closeHosting: (context: SpatialContextV1, instanceId: string) => Promise<DomeHostingView>;
  submitSessionInput: (
    context: SpatialContextV1,
    instanceId: string,
    sequence: number,
    input: DomeSessionInputKindV1
  ) => Promise<DomePhysicsSnapshotV1>;
  moveRoom: (
    moveId: string,
    roomId: string,
    targetContext: SpatialContextV1
  ) => Promise<void>;
  listConnections: (context: SpatialContextV1) => Promise<DomeConnectionTopologyView>;
  createConnectionProposal: (
    proposalId: string,
    context: SpatialContextV1,
    proposerInstanceId: string,
    receiverInstanceId: string,
    direction: DomeDirection
  ) => Promise<DomeConnectionProposalView>;
  acceptConnectionProposal: (
    context: SpatialContextV1,
    proposalId: string
  ) => Promise<DomeConnectionView>;
  withdrawConnectionProposal: (
    context: SpatialContextV1,
    proposalId: string
  ) => Promise<DomeConnectionProposalView>;
  revokeConnection: (
    context: SpatialContextV1,
    connectionId: string
  ) => Promise<DomeConnectionView>;
  refresh: () => Promise<void>;
};
