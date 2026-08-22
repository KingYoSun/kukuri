import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ColumnCanvas } from './ColumnCanvas';
import { ColumnSurface } from './ColumnSurface';

describe('ColumnCanvas', () => {
  beforeEach(() => {
    Element.prototype.scrollIntoView = vi.fn();
  });

  it('announces the active, pinned, transient, position, and span state', () => {
    render(
      <ColumnCanvas activeColumnId='timeline-initial' onActivateColumn={() => undefined}>
        <ColumnSurface
          columnId='timeline-initial'
          title='Timeline'
          scopeLabel='Public · kukuri:topic:demo'
          position={1}
          total={1}
          span={1}
          active
          pinned={false}
        >
          Timeline body
        </ColumnSurface>
      </ColumnCanvas>
    );

    const column = screen.getByRole('region', { name: /Timeline Column/ });
    expect(column).toHaveAttribute('aria-current', 'true');
    expect(column).toHaveAttribute('data-span', '1');
    expect(column).toHaveAttribute('data-transient', 'true');
    expect(column).toHaveAccessibleName(/Column 1 of 1/);
    expect(column).toHaveAccessibleName(/1 span/);
    expect(screen.getByText('Active')).toBeVisible();
    expect(screen.getByText('Temporary')).toBeVisible();
  });

  it('activates a focused Column and scrolls a programmatically active Column into view', async () => {
    const user = userEvent.setup();
    const onActivateColumn = vi.fn();
    const { rerender } = render(
      <ColumnCanvas activeColumnId='timeline-initial' onActivateColumn={onActivateColumn}>
        <ColumnSurface
          columnId='timeline-initial'
          title='Timeline'
          scopeLabel='Public'
          position={1}
          total={2}
          span={1}
          active
          pinned
        >
          <button type='button'>Timeline action</button>
        </ColumnSurface>
        <ColumnSurface
          columnId='thread-1'
          title='Thread'
          scopeLabel='Launch planning'
          position={2}
          total={2}
          span={1}
          active={false}
          pinned={false}
        >
          <button type='button'>Thread action</button>
        </ColumnSurface>
      </ColumnCanvas>
    );

    await user.click(screen.getByRole('button', { name: 'Thread action' }));
    expect(onActivateColumn).toHaveBeenCalledWith('thread-1');

    rerender(
      <ColumnCanvas activeColumnId='thread-1' onActivateColumn={onActivateColumn}>
        <ColumnSurface
          columnId='timeline-initial'
          title='Timeline'
          scopeLabel='Public'
          position={1}
          total={2}
          span={1}
          active={false}
          pinned
        >
          Timeline body
        </ColumnSurface>
        <ColumnSurface
          columnId='thread-1'
          title='Thread'
          scopeLabel='Launch planning'
          position={2}
          total={2}
          span={1}
          active
          pinned={false}
        >
          Thread body
        </ColumnSurface>
      </ColumnCanvas>
    );

    expect(Element.prototype.scrollIntoView).toHaveBeenCalledWith({
      block: 'nearest',
      inline: 'nearest',
    });
  });
});
