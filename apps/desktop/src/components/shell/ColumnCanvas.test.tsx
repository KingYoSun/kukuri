import { act, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { ColumnCanvas } from './ColumnCanvas';
import { ColumnContextSelect } from './ColumnContextSelect';
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

  it('does not activate an inactive Column when its context selector is used', async () => {
    const user = userEvent.setup();
    const onActivateColumn = vi.fn();
    render(
      <ColumnCanvas activeColumnId='profile-1' onActivateColumn={onActivateColumn}>
        <ColumnSurface
          columnId='timeline-1'
          title='Timeline'
          scopeLabel='general'
          scopeControl={
            <ColumnContextSelect
              label='Timeline topic'
              value='general'
              options={[
                { value: 'general', label: 'general' },
                { value: 'dev', label: 'dev' },
              ]}
              onChange={() => undefined}
            />
          }
          position={1}
          total={2}
          span={1}
          active={false}
          pinned
        >
          Timeline body
        </ColumnSurface>
        <ColumnSurface
          columnId='profile-1'
          title='Profile'
          scopeLabel='general'
          position={2}
          total={2}
          span={1}
          active
          pinned
        >
          Profile body
        </ColumnSurface>
      </ColumnCanvas>
    );

    await user.selectOptions(screen.getByRole('combobox', { name: 'Timeline topic' }), 'dev');
    expect(onActivateColumn).not.toHaveBeenCalled();
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

  it('uses only the dedicated grip as a pointer reorder target', () => {
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
      'data-column-drag-grip'
    );
    expect(screen.getByRole('region', { name: /Metaverse Column/ })).not.toHaveAttribute(
      'draggable'
    );
    expect(screen.getByText('Selectable scene description')).not.toHaveAttribute('draggable');
  });

  it('moves the full Column with pointer input and ignores a grip click', () => {
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
    const setPointerCapture = vi.fn();
    const releasePointerCapture = vi.fn();
    Object.defineProperty(canvas, 'setPointerCapture', { value: setPointerCapture });
    Object.defineProperty(canvas, 'releasePointerCapture', { value: releasePointerCapture });
    Object.defineProperty(canvas, 'hasPointerCapture', { value: vi.fn(() => true) });
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
    const grip = screen.getByRole('button', { name: /Move stream Column/i });
    fireEvent.pointerDown(grip, {
      pointerId: 7,
      button: 0,
      isPrimary: true,
      clientX: 350,
      clientY: 40,
    });
    fireEvent.pointerUp(canvas, { pointerId: 7, clientX: 350, clientY: 40 });
    expect(onMoveColumn).not.toHaveBeenCalled();

    fireEvent.pointerDown(grip, {
      pointerId: 8,
      button: 0,
      isPrimary: true,
      clientX: 350,
      clientY: 40,
    });
    fireEvent.pointerMove(canvas, { pointerId: 8, clientX: 850, clientY: 40 });
    expect(screen.getByRole('separator', { name: 'Drop Column at position 3' })).toBeVisible();
    expect(columns[1]).toHaveAttribute('data-dragging', 'true');
    fireEvent.pointerUp(canvas, { pointerId: 8, clientX: 850, clientY: 40 });

    expect(onMoveColumn).toHaveBeenCalledWith('stream', 2);
    expect(onMoveColumn).toHaveBeenCalledTimes(1);
    expect(columns[1]).not.toHaveAttribute('data-dragging');
    expect(setPointerCapture).toHaveBeenCalledWith(7);
    expect(setPointerCapture).toHaveBeenCalledWith(8);
    expect(releasePointerCapture).toHaveBeenCalledWith(7);
    expect(releasePointerCapture).toHaveBeenCalledWith(8);

    fireEvent.pointerDown(grip, {
      pointerId: 9,
      button: 0,
      isPrimary: true,
      clientX: 350,
      clientY: 40,
    });
    fireEvent.pointerMove(canvas, { pointerId: 9, clientX: 850, clientY: 40 });
    fireEvent.pointerCancel(canvas, { pointerId: 9 });
    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
    expect(columns[1]).not.toHaveAttribute('data-dragging');
    expect(onMoveColumn).toHaveBeenCalledTimes(1);

    fireEvent.pointerDown(grip, {
      pointerId: 10,
      button: 0,
      isPrimary: true,
      clientX: 350,
      clientY: 40,
    });
    fireEvent.pointerMove(canvas, { pointerId: 10, clientX: 850, clientY: 40 });
    fireEvent.keyDown(window, { key: 'Escape' });
    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
    expect(columns[1]).not.toHaveAttribute('data-dragging');
    expect(onMoveColumn).toHaveBeenCalledTimes(1);
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

  function stubMobileMatchMedia() {
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
  }

  function stubMobilePageGeometry(container: HTMLElement, scrollLeft: number) {
    const canvas = container.querySelector('.shell-column-canvas') as HTMLElement;
    Object.defineProperty(canvas, 'clientWidth', { value: 390 });
    Object.defineProperty(canvas, 'scrollLeft', { value: scrollLeft, writable: true });
    Array.from(container.querySelectorAll<HTMLElement>('[data-column-id]')).forEach(
      (column, index) => {
        Object.defineProperty(column, 'offsetLeft', { value: index * 390 });
        Object.defineProperty(column, 'offsetWidth', { value: 390 });
      }
    );
    return canvas;
  }

  it('syncs the nearest mobile snap page after settle and offers direct indicator jumps', async () => {
    vi.useFakeTimers();
    stubMobileMatchMedia();
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
    const canvas = stubMobilePageGeometry(container, 390);

    // 実機の user scroll は wheel / touch 入力から始まる。mount 直後の programmatic
    // scroll ガードを user 入力で解除してから settle を検証する。
    fireEvent.wheel(canvas);
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

  it('ignores a mobile settle that fires while a route-driven activation is still scrolling', async () => {
    // hash-routing narrow flake の再現: goBack の route 投影が Timeline を activate し
    // smooth scroll が始まった直後、CI 負荷で scroll が 120ms 以上停滞すると settle が
    // 移動元 Column を user scroll と誤認して再 activate + route 同期していた。
    vi.useFakeTimers();
    stubMobileMatchMedia();
    const onActivateColumn = vi.fn();
    const surfaces = (activeColumnId: string) =>
      ['timeline', 'thread', 'profile'].map((id, index) => (
        <ColumnSurface
          key={id}
          columnId={id}
          title={id}
          scopeLabel='Public'
          position={index + 1}
          total={3}
          span={1}
          active={id === activeColumnId}
          pinned
        >
          {id}
        </ColumnSurface>
      ));
    const { container, rerender } = render(
      <ColumnCanvas
        activeColumnId='thread'
        columnIds={['timeline', 'thread', 'profile']}
        onActivateColumn={onActivateColumn}
      >
        {surfaces('thread')}
      </ColumnCanvas>
    );
    const canvas = stubMobilePageGeometry(container, 390);

    // goBack 相当: route 投影が timeline を activate し programmatic scroll が始まる。
    rerender(
      <ColumnCanvas
        activeColumnId='timeline'
        columnIds={['timeline', 'thread', 'profile']}
        onActivateColumn={onActivateColumn}
      >
        {surfaces('timeline')}
      </ColumnCanvas>
    );

    // scroll 位置がまだ thread ページのまま settle が発火しても activate しない。
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).not.toHaveBeenCalled();

    // 目的地到達の settle でガードが解除される(activate は不要)。
    canvas.scrollLeft = 0;
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).not.toHaveBeenCalled();

    // 解除後の user scroll paging は従来どおり動く。
    canvas.scrollLeft = 780;
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).toHaveBeenCalledWith('profile', true);
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it('lets wheel input take paging over from a route-driven programmatic scroll', async () => {
    vi.useFakeTimers();
    stubMobileMatchMedia();
    const onActivateColumn = vi.fn();
    const surfaces = (activeColumnId: string) =>
      ['timeline', 'thread', 'profile'].map((id, index) => (
        <ColumnSurface
          key={id}
          columnId={id}
          title={id}
          scopeLabel='Public'
          position={index + 1}
          total={3}
          span={1}
          active={id === activeColumnId}
          pinned
        >
          {id}
        </ColumnSurface>
      ));
    const { container, rerender } = render(
      <ColumnCanvas
        activeColumnId='thread'
        columnIds={['timeline', 'thread', 'profile']}
        onActivateColumn={onActivateColumn}
      >
        {surfaces('thread')}
      </ColumnCanvas>
    );
    const canvas = stubMobilePageGeometry(container, 390);
    rerender(
      <ColumnCanvas
        activeColumnId='timeline'
        columnIds={['timeline', 'thread', 'profile']}
        onActivateColumn={onActivateColumn}
      >
        {surfaces('timeline')}
      </ColumnCanvas>
    );

    // wheel 入力はユーザーの引き継ぎ。programmatic scroll ガードを解除し settle を有効化する。
    fireEvent.wheel(canvas);
    canvas.scrollLeft = 780;
    fireEvent.scroll(canvas);
    await act(async () => vi.advanceTimersByTime(120));
    expect(onActivateColumn).toHaveBeenCalledWith('profile', true);
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
