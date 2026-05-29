import { describe, expect, test } from 'vitest';

import {
  isNewerRemoteTransform,
  avatarAnimationForInput,
  normalizeAvatarAnimationState,
  stepAvatarJump,
  type AvatarTransform,
} from './MetaverseSceneModel';

describe('metaverse avatar animation state', () => {
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
});
