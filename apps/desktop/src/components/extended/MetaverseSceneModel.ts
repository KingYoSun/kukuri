import type { SharedRoomObjectV1 } from '@/lib/api';

export type MetaverseVec3 = [number, number, number];

export type AvatarTransform = {
  roomId: string;
  peerId: string;
  seq: number;
  position: MetaverseVec3;
  rotation: MetaverseVec3;
  animation: 'idle' | 'walk';
  sentAt: number;
};

export type RoomChatMessage = {
  roomId: string;
  messageId: string;
  authorPeerId: string;
  body: string;
  createdAt: number;
};

export type MetaverseRoomEvent =
  | { type: 'presence.join'; roomId: string; peerId: string; at: number }
  | { type: 'avatar.transform'; transform: AvatarTransform }
  | { type: 'chat.message'; message: RoomChatMessage }
  | { type: 'object.update'; roomId: string; object: SharedRoomObjectV1 };

export type AvatarAssetStatus = 'loading' | 'sample-vrm' | 'blob-vrm' | 'fallback-primitive';

export const DEFAULT_SHARED_OBJECT: SharedRoomObjectV1 = {
  object_id: 'mvp-object-1',
  asset_ref: null,
  primitive_fallback: 'cube',
  position: [0, 50, -240],
  rotation: [0, 0, 0],
  scale: [100, 100, 100],
  updated_by: '',
  updated_at: 0,
};

export const DEFAULT_AVATAR_ASSET_URL = '/avatar_sample_a.vrm';
