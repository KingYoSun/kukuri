import { describe, expect, test } from 'vitest';

import { avatarAnimationForInput, normalizeAvatarAnimationState } from './MetaverseSceneModel';

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
});
