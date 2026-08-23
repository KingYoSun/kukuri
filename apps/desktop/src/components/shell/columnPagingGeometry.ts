export type ColumnPageRect = {
  id: string;
  left: number;
  width: number;
};

export function nearestColumnToViewportCenter(
  columns: ColumnPageRect[],
  scrollLeft: number,
  viewportWidth: number
): string | null {
  if (columns.length === 0 || !Number.isFinite(viewportWidth) || viewportWidth <= 0) return null;
  const viewportCenter = scrollLeft + viewportWidth / 2;
  let nearest: ColumnPageRect | null = null;
  let nearestDistance = Number.POSITIVE_INFINITY;
  for (const column of columns) {
    if (!Number.isFinite(column.left) || !Number.isFinite(column.width) || column.width <= 0) continue;
    const distance = Math.abs(column.left + column.width / 2 - viewportCenter);
    if (distance < nearestDistance) {
      nearest = column;
      nearestDistance = distance;
    }
  }
  return nearest?.id ?? null;
}
