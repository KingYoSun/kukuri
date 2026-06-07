import { describe, expect, test } from 'vitest';

import {
  METAVERSE_AVATAR_IDLE_SEND_INTERVAL_MS,
  METAVERSE_AVATAR_MOVING_SEND_INTERVAL_MS,
  METAVERSE_REMOTE_AVATAR_POSITION_SMOOTHING_SECONDS,
  METAVERSE_ROOM_STALE_MS,
  isNewerRemoteTransform,
  isNewerSharedObject,
  avatarAnimationForInput,
  mergeRoomChatMessages,
  normalizeAvatarAnimationState,
  remoteAvatarYawDegrees,
  stepAvatarJump,
  type AvatarTransform,
} from './MetaverseSceneModel';

describe('metaverse avatar animation state', () => {
  test('uses low-latency transform intervals and a longer stale threshold', () => {
    expect(METAVERSE_AVATAR_MOVING_SEND_INTERVAL_MS).toBe(30);
    expect(METAVERSE_AVATAR_IDLE_SEND_INTERVAL_MS).toBe(150);
    expect(METAVERSE_REMOTE_AVATAR_POSITION_SMOOTHING_SECONDS).toBe(0.14);
    expect(METAVERSE_ROOM_STALE_MS).toBe(45_000);
  });

  test('uses the received remote yaw without smoothing', () => {
    expect(remoteAvatarYawDegrees([0, 135, 0])).toBe(135);
  });

  test('derives keyboard animation state', () => {
    expect(avatarAnimationForInput([], false)).toBe('idle');
    expect(avatarAnimationForInput(['w'], false)).toBe('walk');
    expect(avatarAnimationForInput(['Shift', 'ArrowUp'], false)).toBe('sprint');
    expect(avatarAnimationForInput(['w'], true)).toBe('sitting');
  });

  test('normalizes shared animation states', () => {
    expect(normalizeAvatarAnimationState('idle')).toBe('idle');
    expect(normalizeAvatarAnimationState('walk')).toBe('walk');
    expect(normalizeAvatarAnimationState('sprint')).toBe('sprint');
    expect(normalizeAvatarAnimationState('jump')).toBe('jump');
    expect(normalizeAvatarAnimationState('sitting')).toBe('sitting');
    expect(normalizeAvatarAnimationState('Sittiing')).toBe('idle');
    expect(normalizeAvatarAnimationState(null)).toBe('idle');
  });

  test('jump movement changes height and lands on the ground', () => {
    const launched = stepAvatarJump([0, 0, 0], { verticalVelocity: 0, grounded: true }, 0.1, true);
    expect(launched.position[1]).toBeGreaterThan(0);
    expect(launched.physics.grounded).toBe(false);

    let step = launched;
    for (let index = 0; index < 20; index += 1) {
      step = stepAvatarJump(step.position, step.physics, 0.1, false);
    }
    expect(step.position[1]).toBe(0);
    expect(step.physics.grounded).toBe(true);
  });

  test('detects stale remote transforms by seq and timestamp', () => {
    const current: AvatarTransform = {
      roomId: 'room',
      peerId: 'peer',
      seq: 3,
      position: [0, 0, 0],
      rotation: [0, 0, 0],
      animation: 'idle',
      sentAt: 300,
    };
    expect(isNewerRemoteTransform(current, { ...current, seq: 2, sentAt: 400 })).toBe(false);
    expect(isNewerRemoteTransform(current, { ...current, seq: 4, sentAt: 250 })).toBe(true);
    expect(isNewerRemoteTransform(current, { ...current, sentAt: 301 })).toBe(true);
  });

  test('accepts only newer shared object updates', () => {
    const current = {
      object_id: 'mvp-object-1',
      asset_ref: null,
      primitive_fallback: 'cube' as const,
      position: [0, 50, -240] as [number, number, number],
      rotation: [0, 0, 0] as [number, number, number],
      scale: [100, 100, 100] as [number, number, number],
      updated_by: 'peer-a',
      updated_at: 20,
    };

    expect(isNewerSharedObject(current, { ...current, position: [0, 50, -290], updated_at: 19 })).toBe(false);
    expect(isNewerSharedObject(current, { ...current, position: [0, 50, -290], updated_at: 21 })).toBe(true);
  });

  test('deduplicates and caps room chat messages', () => {
    const current = Array.from({ length: 99 }, (_, index) => ({
      roomId: 'room',
      messageId: `chat-${index}`,
      authorPeerId: 'peer',
      body: `message ${index}`,
      createdAt: index,
    }));
    const merged = mergeRoomChatMessages(current, [
      {
        roomId: 'room',
        messageId: 'chat-98',
        authorPeerId: 'peer',
        body: 'message 98 edited by arrival order',
        createdAt: 98,
      },
      {
        roomId: 'room',
        messageId: 'chat-100',
        authorPeerId: 'peer',
        body: 'message 100',
        createdAt: 100,
      },
      {
        roomId: 'room',
        messageId: 'chat-101',
        authorPeerId: 'peer',
        body: 'message 101',
        createdAt: 101,
      },
    ]);

    expect(merged).toHaveLength(100);
    expect(merged[0].messageId).toBe('chat-1');
    expect(merged.find((message) => message.messageId === 'chat-98')?.body).toBe(
      'message 98 edited by arrival order'
    );
    expect(merged.at(-1)?.messageId).toBe('chat-101');
  });
});
