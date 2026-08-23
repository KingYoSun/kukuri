import { describe, expect, it } from 'vitest';

import {
  nearestColumnToViewportCenter,
  type ColumnPageRect,
} from './columnPagingGeometry';

const columns: ColumnPageRect[] = [
  { id: 'timeline', left: 0, width: 390 },
  { id: 'thread', left: 390, width: 390 },
  { id: 'profile', left: 780, width: 390 },
];

describe('mobile Column paging geometry', () => {
  it('selects the page nearest the viewport center after scrolling settles', () => {
    expect(nearestColumnToViewportCenter(columns, 0, 390)).toBe('timeline');
    expect(nearestColumnToViewportCenter(columns, 380, 390)).toBe('thread');
    expect(nearestColumnToViewportCenter(columns, 790, 390)).toBe('profile');
  });

  it('uses stable document order for an exact tie and rejects invalid geometry', () => {
    expect(nearestColumnToViewportCenter(columns, 195, 390)).toBe('timeline');
    expect(nearestColumnToViewportCenter([], 0, 390)).toBeNull();
    expect(nearestColumnToViewportCenter(columns, 0, 0)).toBeNull();
  });
});
