import { describe, expect, it } from 'vitest';

import { columnSwipeTargetIndex } from './columnSwipeGesture';

describe('columnSwipeTargetIndex', () => {
  it('moves one page in the horizontal swipe direction without wrapping', () => {
    expect(
      columnSwipeTargetIndex({ activeIndex: 1, columnCount: 3, deltaX: -72, deltaY: 8 })
    ).toBe(2);
    expect(
      columnSwipeTargetIndex({ activeIndex: 1, columnCount: 3, deltaX: 72, deltaY: 8 })
    ).toBe(0);
    expect(
      columnSwipeTargetIndex({ activeIndex: 0, columnCount: 3, deltaX: 72, deltaY: 0 })
    ).toBeNull();
  });

  it('ignores short and vertically dominant gestures', () => {
    expect(
      columnSwipeTargetIndex({ activeIndex: 1, columnCount: 3, deltaX: -30, deltaY: 0 })
    ).toBeNull();
    expect(
      columnSwipeTargetIndex({ activeIndex: 1, columnCount: 3, deltaX: -64, deltaY: 60 })
    ).toBeNull();
  });
});
