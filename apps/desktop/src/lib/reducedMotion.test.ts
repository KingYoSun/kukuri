import { afterEach, describe, expect, it, vi } from 'vitest';

import { prefersReducedMotion } from './reducedMotion';

// matchMedia を任意の結果で stub する(prefers-reduced-motion の query のみ対象)。
function stubMatchMedia(matches: boolean) {
  vi.stubGlobal(
    'matchMedia',
    vi.fn((query: string) => ({
      matches: query === '(prefers-reduced-motion: reduce)' ? matches : false,
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }))
  );
}

describe('prefersReducedMotion', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    delete document.documentElement.dataset.reducedMotion;
  });

  it('returns false when the OS preference is off and no data attribute is set', () => {
    stubMatchMedia(false);
    expect(prefersReducedMotion()).toBe(false);
  });

  it('returns true when the OS preference requests reduced motion', () => {
    stubMatchMedia(true);
    expect(prefersReducedMotion()).toBe(true);
  });

  it('returns true when a review surface sets data-reduced-motion="reduce"', () => {
    stubMatchMedia(false);
    document.documentElement.dataset.reducedMotion = 'reduce';
    expect(prefersReducedMotion()).toBe(true);
  });

  it('keeps the OS preference authoritative when both signals request reduced motion', () => {
    stubMatchMedia(true);
    document.documentElement.dataset.reducedMotion = 'reduce';
    expect(prefersReducedMotion()).toBe(true);
  });

  it('ignores data attribute values other than "reduce"', () => {
    stubMatchMedia(false);
    document.documentElement.dataset.reducedMotion = 'full';
    expect(prefersReducedMotion()).toBe(false);
  });

  it('returns false when matchMedia is unavailable and no data attribute is set', () => {
    vi.stubGlobal('matchMedia', undefined);
    expect(prefersReducedMotion()).toBe(false);
  });
});
