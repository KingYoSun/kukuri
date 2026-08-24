type ColumnSwipeTargetInput = {
  activeIndex: number;
  columnCount: number;
  deltaX: number;
  deltaY: number;
};

const MIN_SWIPE_DISTANCE_PX = 56;
const HORIZONTAL_DOMINANCE_RATIO = 1.25;

export function columnSwipeTargetIndex({
  activeIndex,
  columnCount,
  deltaX,
  deltaY,
}: ColumnSwipeTargetInput): number | null {
  if (
    activeIndex < 0 ||
    columnCount < 2 ||
    Math.abs(deltaX) < MIN_SWIPE_DISTANCE_PX ||
    Math.abs(deltaX) < Math.abs(deltaY) * HORIZONTAL_DOMINANCE_RATIO
  ) {
    return null;
  }
  const targetIndex = activeIndex + (deltaX < 0 ? 1 : -1);
  return targetIndex >= 0 && targetIndex < columnCount ? targetIndex : null;
}
