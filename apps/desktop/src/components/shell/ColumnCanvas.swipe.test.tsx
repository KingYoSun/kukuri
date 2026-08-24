import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, expect, test, vi } from 'vitest';

import { ColumnCanvas } from './ColumnCanvas';
import { ColumnSurface } from './ColumnSurface';

afterEach(() => vi.unstubAllGlobals());

test('mobile edge and page-indicator swipes activate adjacent columns', () => {
  vi.stubGlobal(
    'matchMedia',
    vi.fn((query: string) => ({
      matches: query === '(max-width: 759px)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    }))
  );
  const onActivateColumn = vi.fn();
  const { container } = render(
    <ColumnCanvas
      activeColumnId='thread'
      columnIds={['timeline', 'thread', 'profile']}
      onActivateColumn={onActivateColumn}
    >
      {['timeline', 'thread', 'profile'].map((id, index) => (
        <ColumnSurface
          key={id}
          active={id === 'thread'}
          columnId={id}
          pinned
          position={index + 1}
          scopeLabel='Public'
          span={1}
          title={id}
          total={3}
        >
          {id}
        </ColumnSurface>
      ))}
    </ColumnCanvas>
  );
  const canvas = container.querySelector('.shell-column-canvas') as HTMLElement;
  vi.spyOn(canvas, 'getBoundingClientRect').mockReturnValue({
    left: 0,
    right: 390,
    top: 0,
    bottom: 800,
    width: 390,
    height: 800,
    x: 0,
    y: 0,
    toJSON: () => ({}),
  });

  const indicator = screen.getByRole('navigation', { name: 'Column pages' });
  fireEvent.pointerDown(indicator, { pointerId: 1, clientX: 210, clientY: 700 });
  fireEvent.pointerUp(indicator, { pointerId: 1, clientX: 120, clientY: 706 });
  expect(onActivateColumn).toHaveBeenLastCalledWith('profile', true);

  fireEvent.pointerDown(canvas, { pointerId: 2, clientX: 8, clientY: 400 });
  fireEvent.pointerUp(canvas, { pointerId: 2, clientX: 88, clientY: 405 });
  expect(onActivateColumn).toHaveBeenLastCalledWith('timeline', true);

  onActivateColumn.mockClear();
  fireEvent.pointerDown(indicator, { pointerId: 3, clientX: 210, clientY: 700 });
  fireEvent.pointerDown(indicator, { pointerId: 4, clientX: 210, clientY: 700 });
  fireEvent.pointerUp(indicator, { pointerId: 4, clientX: 120, clientY: 706 });
  expect(onActivateColumn).not.toHaveBeenCalled();
  fireEvent.pointerUp(indicator, { pointerId: 3, clientX: 120, clientY: 706 });
  expect(onActivateColumn).toHaveBeenCalledWith('profile', true);
});
