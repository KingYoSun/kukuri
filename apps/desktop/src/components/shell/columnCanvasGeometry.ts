const EDGE_SCROLL_ZONE_PX = 56;

export function columnCanvasEdgeScrollDirection({
  clientX,
  left,
  right,
  scrollLeft,
  clientWidth,
  scrollWidth,
}: {
  clientX: number;
  left: number;
  right: number;
  scrollLeft: number;
  clientWidth: number;
  scrollWidth: number;
}): -1 | 0 | 1 {
  if (clientX <= left + EDGE_SCROLL_ZONE_PX && scrollLeft > 0) return -1;
  if (
    clientX >= right - EDGE_SCROLL_ZONE_PX &&
    scrollLeft + clientWidth < scrollWidth
  ) {
    return 1;
  }
  return 0;
}
