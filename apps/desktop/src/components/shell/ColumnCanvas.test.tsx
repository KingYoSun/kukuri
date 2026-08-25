import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ColumnCanvas } from './ColumnCanvas';
import { columnCanvasEdgeScrollDirection } from './columnCanvasGeometry';
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
    expect(onActivateColumn).toHaveBeenCalledWith('thread-1', false);

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

  it('exposes same-header pin and close controls without changing Column activation', async () => {
    const user = userEvent.setup();
    const onPinnedChange = vi.fn();
    const onClose = vi.fn();
    render(
      <ColumnCanvas activeColumnId='thread-1' onActivateColumn={() => undefined}>
        <ColumnSurface
          columnId='thread-1'
          title='Thread'
          scopeLabel='Launch planning'
          position={1}
          total={2}
          span={1}
          active
          pinned={false}
          onPinnedChange={onPinnedChange}
          onClose={onClose}
        >
          Thread body
        </ColumnSurface>
      </ColumnCanvas>
    );

    await user.click(screen.getByRole('button', { name: 'Pin Thread' }));
    await user.click(screen.getByRole('button', { name: 'Close Thread' }));
    expect(onPinnedChange).toHaveBeenCalledWith(true);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('offers keyboard reorder and allowed span choices from the Column menu', async () => {
    const user = userEvent.setup();
    const onMoveLeft = vi.fn();
    const onMoveRight = vi.fn();
    const onSpanChange = vi.fn();
    render(
      <ColumnCanvas activeColumnId='stream-1' onActivateColumn={() => undefined}>
        <ColumnSurface
          columnId='stream-1'
          title='Stream'
          scopeLabel='Public'
          position={2}
          total={3}
          span={2}
          spanOptions={[1, 2]}
          active
          pinned
          onMoveLeft={onMoveLeft}
          onMoveRight={onMoveRight}
          onSpanChange={onSpanChange}
        >
          Stream body
        </ColumnSurface>
      </ColumnCanvas>
    );

    await user.click(screen.getByRole('button', { name: 'Open Stream menu' }));
    expect(screen.getByRole('menu', { name: 'Stream actions' })).toBeVisible();
    expect(screen.getByRole('menuitemradio', { name: '1 span' })).not.toBeChecked();
    expect(screen.getByRole('menuitemradio', { name: '2 spans' })).toBeChecked();

    await user.click(screen.getByRole('menuitem', { name: 'Move Stream left' }));
    expect(onMoveLeft).toHaveBeenCalledTimes(1);
    await user.click(screen.getByRole('button', { name: 'Open Stream menu' }));
    await user.click(screen.getByRole('menuitemradio', { name: '1 span' }));
    expect(onSpanChange).toHaveBeenCalledWith(1);
    expect(onMoveRight).not.toHaveBeenCalled();
  });

  it('reaches reorder and span actions from the Column menu with the keyboard only', async () => {
    const user = userEvent.setup();
    const onMoveLeft = vi.fn();
    const onMoveRight = vi.fn();
    const onSpanChange = vi.fn();
    render(
      <ColumnCanvas activeColumnId='stream-1' onActivateColumn={() => undefined}>
        <ColumnSurface
          columnId='stream-1'
          title='Stream'
          scopeLabel='Public'
          position={2}
          total={3}
          span={2}
          spanOptions={[1, 2]}
          active
          pinned
          onMoveLeft={onMoveLeft}
          onMoveRight={onMoveRight}
          onSpanChange={onSpanChange}
        >
          <button type='button'>Stream body action</button>
        </ColumnSurface>
      </ColumnCanvas>
    );

    const trigger = screen.getByRole('button', { name: 'Open Stream menu' });
    trigger.focus();
    await user.keyboard('{Enter}');
    expect(screen.getByRole('menu', { name: 'Stream actions' })).toBeVisible();
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream left' })
    );

    await user.keyboard('{ArrowDown}');
    expect(document.activeElement).toBe(
      screen.getByRole('menuitem', { name: 'Move Stream right' })
    );
    await user.keyboard('{Enter}');
    expect(onMoveRight).toHaveBeenCalledTimes(1);
    expect(onMoveLeft).not.toHaveBeenCalled();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Open Stream menu' }));

    await user.keyboard('{ArrowDown}');
    await user.keyboard('{ArrowDown}{ArrowDown}');
    expect(document.activeElement).toBe(screen.getByRole('menuitemradio', { name: '1 span' }));
    await user.keyboard('{Enter}');
    expect(onSpanChange).toHaveBeenCalledWith(1);
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('uses only the dedicated grip as a draggable target', () => {
    render(
      <ColumnCanvas activeColumnId='metaverse-1' onActivateColumn={() => undefined}>
        <ColumnSurface
          columnId='metaverse-1'
          title='Metaverse'
          scopeLabel='Room'
          position={1}
          total={1}
          span={3}
          active
          pinned
        >
          <p>Selectable scene description</p>
        </ColumnSurface>
      </ColumnCanvas>
    );

    expect(screen.getByRole('button', { name: 'Move Metaverse Column' })).toHaveAttribute(
      'draggable',
      'true'
    );
    expect(screen.getByRole('region', { name: /Metaverse Column/ })).not.toHaveAttribute(
      'draggable'
    );
    expect(screen.getByText('Selectable scene description')).not.toHaveAttribute('draggable');
  });

  it('moves the full Column to the drop index', () => {
    const onMoveColumn = vi.fn();
    const { container } = render(
      <ColumnCanvas
        activeColumnId='timeline'
        onActivateColumn={() => undefined}
        onMoveColumn={onMoveColumn}
      >
        {['timeline', 'stream', 'profile'].map((id, index) => (
          <ColumnSurface
            key={id}
            columnId={id}
            title={id}
            scopeLabel='Public'
            position={index + 1}
            total={3}
            span={id === 'stream' ? 2 : 1}
            active={id === 'timeline'}
            pinned
          >
            {id} body
          </ColumnSurface>
        ))}
      </ColumnCanvas>
    );
    const canvas = container.querySelector('.shell-column-canvas') as HTMLElement;
    const columns = Array.from(
      container.querySelectorAll<HTMLElement>('[data-column-id]')
    );
    Object.defineProperty(canvas, 'getBoundingClientRect', {
      value: () => ({ left: 0, right: 900, top: 0, bottom: 500, width: 900, height: 500 }),
    });
    columns.forEach((column, index) => {
      Object.defineProperty(column, 'offsetLeft', { value: index * 300 });
      Object.defineProperty(column, 'offsetWidth', { value: 280 });
      Object.defineProperty(column, 'getBoundingClientRect', {
        value: () => ({
          left: index * 300,
          right: index * 300 + 280,
          top: 0,
          bottom: 500,
          width: 280,
          height: 500,
        }),
      });
    });
    const dataTransfer = {
      effectAllowed: '',
      dropEffect: '',
      setData: vi.fn(),
      getData: vi.fn(() => 'stream'),
      setDragImage: vi.fn(),
    };

    fireEvent.dragStart(screen.getByRole('button', { name: /Move stream Column/i }), {
      dataTransfer,
      clientX: 350,
    });
    fireEvent.dragOver(canvas, { dataTransfer, clientX: 850 });
    expect(screen.getByRole('separator', { name: 'Drop Column at position 3' })).toBeVisible();
    fireEvent.drop(canvas, { dataTransfer, clientX: 850 });

    expect(onMoveColumn).toHaveBeenCalledWith('stream', 2);
  });

  it('starts edge auto-scroll only in a direction with remaining overflow', () => {
    const base = {
      left: 0,
      right: 500,
      scrollLeft: 100,
      clientWidth: 500,
      scrollWidth: 1000,
    };
    expect(columnCanvasEdgeScrollDirection({ ...base, clientX: 20 })).toBe(-1);
    expect(columnCanvasEdgeScrollDirection({ ...base, clientX: 490 })).toBe(1);
    expect(columnCanvasEdgeScrollDirection({ ...base, clientX: 250 })).toBe(0);
    expect(
      columnCanvasEdgeScrollDirection({ ...base, clientX: 20, scrollLeft: 0 })
    ).toBe(0);
    expect(
      columnCanvasEdgeScrollDirection({ ...base, clientX: 490, scrollLeft: 500 })
    ).toBe(0);
  });

  it('syncs the nearest mobile snap page after settle and offers direct indicator jumps', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('matchMedia', vi.fn((query: string) => ({
      matches: query === '(max-width: 759px)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })));
    const onActivateColumn = vi.fn();
    const { container } = render(
      <ColumnCanvas
        activeColumnId='timeline'
        columnIds={['timeline', 'thread', 'profile']}
        onActivateColumn={onActivateColumn}
      >
        {['timeline', 'thread', 'profile'].map((id, index) => (
          <ColumnSurface
            key={id}
            columnId={id}
            title={id}
            scopeLabel='Public'
            position={index + 1}
            total={3}
            span={1}
            active={id === 'timeline'}
            pinned
          >
            {id}
          </ColumnSurface>
        ))}
      </ColumnCanvas>
    );
    const canvas = container.querySelector('.shell-column-canvas') as HTMLElement;
    Object.defineProperty(canvas, 'clientWidth', { value: 390 });
    Object.defineProperty(canvas, 'scrollLeft', { value: 390, writable: true });
    Array.from(container.querySelectorAll<HTMLElement>('[data-column-id]')).forEach(
      (column, index) => {
        Object.defineProperty(column, 'offsetLeft', { value: index * 390 });
        Object.defineProperty(column, 'offsetWidth', { value: 390 });
      }
    );

    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).toHaveBeenCalledWith('thread', true);
    expect(screen.getByText('1 / 3')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Go to Column 3 of 3' }));
    expect(onActivateColumn).toHaveBeenLastCalledWith('profile', true);

    // A delayed settle from the previous page must not override a direct indicator jump.
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).toHaveBeenCalledTimes(2);
    expect(onActivateColumn).toHaveBeenLastCalledWith('profile', true);

    canvas.scrollLeft = 780;
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).toHaveBeenCalledTimes(2);
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('drops smooth scrolling on mobile when a review surface sets data-reduced-motion', () => {
    // OS 設定(matchMedia)は reduce でなくても、data-reduced-motion='reduce' が
    // prefersReducedMotion() 経由で拾われ、mobile の Column 切替 scroll が即時になる。
    vi.stubGlobal('matchMedia', vi.fn((query: string) => ({
      matches: query === '(max-width: 759px)',
      media: query,
      onchange: null,
      addListener: vi.fn(),
      removeListener: vi.fn(),
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })));
    document.documentElement.dataset.reducedMotion = 'reduce';
    const surfaces = (activeColumnId: string) =>
      ['timeline', 'thread'].map((id, index) => (
        <ColumnSurface
          key={id}
          columnId={id}
          title={id}
          scopeLabel='Public'
          position={index + 1}
          total={2}
          span={1}
          active={id === activeColumnId}
          pinned
        >
          {id}
        </ColumnSurface>
      ));
    const { rerender } = render(
      <ColumnCanvas activeColumnId='timeline' onActivateColumn={() => undefined}>
        {surfaces('timeline')}
      </ColumnCanvas>
    );

    rerender(
      <ColumnCanvas activeColumnId='thread' onActivateColumn={() => undefined}>
        {surfaces('thread')}
      </ColumnCanvas>
    );

    expect(Element.prototype.scrollIntoView).toHaveBeenLastCalledWith({
      block: 'nearest',
      inline: 'center',
      behavior: 'auto',
    });
    delete document.documentElement.dataset.reducedMotion;
    vi.unstubAllGlobals();
  });

  it('re-observes replaced column elements when the id set changes without a count change', () => {
    // Issue #765 T3: 同数の transient 置換(Column 数は不変で id 列だけ入れ替わる)でも
    // IntersectionObserver が新しい Column 要素を observe し直し、置換前の id が
    // visible 集合に残留しないことを固定する。
    class MockIntersectionObserver {
      static instances: MockIntersectionObserver[] = [];
      observed = new Set<Element>();
      constructor(
        private readonly callback: IntersectionObserverCallback,
        public readonly options?: IntersectionObserverInit
      ) {
        MockIntersectionObserver.instances.push(this);
      }
      observe(target: Element) {
        this.observed.add(target);
      }
      unobserve(target: Element) {
        this.observed.delete(target);
      }
      disconnect() {
        this.observed.clear();
      }
      takeRecords(): IntersectionObserverEntry[] {
        return [];
      }
      trigger(entries: Array<Pick<IntersectionObserverEntry, 'target' | 'isIntersecting' | 'intersectionRatio'>>) {
        this.callback(
          entries as IntersectionObserverEntry[],
          this as unknown as IntersectionObserver
        );
      }
    }
    vi.stubGlobal('IntersectionObserver', MockIntersectionObserver);

    const onVisibleColumnIdsChange = vi.fn();
    const canvasWith = (ids: string[]) => (
      <ColumnCanvas
        activeColumnId={ids[0]}
        columnIds={ids}
        onActivateColumn={() => undefined}
        onVisibleColumnIdsChange={onVisibleColumnIdsChange}
      >
        {ids.map((id) => (
          <div key={id} data-column-id={id}>
            {id}
          </div>
        ))}
      </ColumnCanvas>
    );
    const { rerender } = render(canvasWith(['timeline-1', 'stream-1']));

    const initialObserver = MockIntersectionObserver.instances.at(-1)!;
    const initialColumns = Array.from(document.querySelectorAll<HTMLElement>('[data-column-id]'));
    expect(initialColumns).toHaveLength(2);
    initialColumns.forEach((column) => {
      expect(initialObserver.observed.has(column)).toBe(true);
    });
    act(() => {
      initialObserver.trigger(
        initialColumns.map((column) => ({
          target: column,
          isIntersecting: true,
          intersectionRatio: 1,
        }))
      );
    });
    expect(onVisibleColumnIdsChange).toHaveBeenLastCalledWith(['timeline-1', 'stream-1']);

    // 同数のまま stream-1 -> game-1 へ置換する(Column 数は 2 のまま)。
    rerender(canvasWith(['timeline-1', 'game-1']));
    const gameColumn = document.querySelector<HTMLElement>('[data-column-id="game-1"]');
    expect(gameColumn).not.toBeNull();

    // 置換後の Column 要素が observe されている(修正前は旧 observer のままで失敗する)。
    const latestObserver = MockIntersectionObserver.instances.at(-1)!;
    expect(latestObserver.observed.has(gameColumn!)).toBe(true);

    // 新要素の可視化が publish され、置換前の id は visible 集合から消える。
    const timelineColumn = document.querySelector<HTMLElement>('[data-column-id="timeline-1"]')!;
    act(() => {
      latestObserver.trigger([
        { target: timelineColumn, isIntersecting: true, intersectionRatio: 1 },
        { target: gameColumn!, isIntersecting: true, intersectionRatio: 1 },
      ]);
    });
    expect(onVisibleColumnIdsChange).toHaveBeenLastCalledWith(['timeline-1', 'game-1']);
    const published = onVisibleColumnIdsChange.mock.calls.map(([ids]) => ids as string[]);
    expect(published.at(-1)).not.toContain('stream-1');

    vi.unstubAllGlobals();
  });
});
