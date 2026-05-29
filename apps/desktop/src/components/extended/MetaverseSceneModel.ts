import type { SharedRoomObjectV1 } from '@/lib/api';

export type MetaverseVec3 = [number, number, number];

export const AVATAR_ANIMATION_STATES = ['idle', 'walk', 'sprint', 'jump', 'sitting'] as const;

export type AvatarAnimationState = (typeof AVATAR_ANIMATION_STATES)[number];

export function normalizeAvatarAnimationState(value: string | null | undefined): AvatarAnimationState {
  if (value === 'idle' || value === 'walk' || value === 'sprint' || value === 'jump' || value === 'sitting') {
    return value;
  }
  return 'idle';
}

export function avatarAnimationForInput(keys: Iterable<string>, sitting: boolean): AvatarAnimationState {
  if (sitting) {
    return 'sitting';
  }
  const keySet = new Set(Array.from(keys, (key) => key.toLowerCase()));
  const moving =
    keySet.has('w') ||
    keySet.has('a') ||
    keySet.has('s') ||
    keySet.has('d') ||
    keySet.has('arrowup') ||
    keySet.has('arrowleft') ||
    keySet.has('arrowdown') ||
    keySet.has('arrowright');
  if (!moving) {
    return 'idle';
  }
  return keySet.has('shift') ? 'sprint' : 'walk';
}

export type AvatarTransform = {
  roomId: string;
  peerId: string;
  seq: number;
  position: MetaverseVec3;
  rotation: MetaverseVec3;
  animation: AvatarAnimationState;
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

export const DEFAULT_AVATAR_ASSET_NAME = 'blumochichi.vrm';
export const DEFAULT_AVATAR_ASSET_URL = `/${DEFAULT_AVATAR_ASSET_NAME}`;
