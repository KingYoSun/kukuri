import type {
  GameRoomView,
  MetaverseAssetRef,
  MetaverseRoomEventView,
  MetaverseRoomEventV1,
} from '@/lib/api';
import type { MetaverseVec3 } from '../MetaverseSceneModel';

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
    kind: 'vrm',
    mime: string,
    name: string,
    dataBase64: string
  ) => Promise<MetaverseAssetRef>;
  getBlobPreviewUrl: (blobHash: string, mime: string) => Promise<string | null>;
  updateRoom: (
    roomId: string,
    status: GameRoomView['status'],
    position: MetaverseVec3,
    rotation: MetaverseVec3,
    scale: MetaverseVec3
  ) => Promise<void>;
  refresh: () => Promise<void>;
};
