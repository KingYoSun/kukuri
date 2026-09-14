import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { ColumnCanvas } from './ColumnCanvas';
import { ColumnSurface } from './ColumnSurface';

afterEach(() => vi.unstubAllGlobals());

describe('Column span resize context', () => {
  function fixture() {
    let notify = () => {};
    vi.stubGlobal('ResizeObserver', class {
      constructor(callback: () => void) { notify = callback; }
      observe() {}
      disconnect() {}
    });
    let bounds = { left: 500, right: 900, width: 400 };
    const { container, rerender } = render(<div />);
    const view = (
      <ColumnCanvas activeColumnId='room' onActivateColumn={() => {}}>
        <ColumnSurface active columnId='room' pinned position={1} total={1} title='Metaverse' scopeLabel='Public' span={1}>
          <input aria-label='Draft' defaultValue='unsent' />
        </ColumnSurface>
      </ColumnCanvas>
    );
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      return { ...(this.classList.contains('shell-column-canvas') ? { left: 0, right: 900, width: 900 } : bounds), top: 0, bottom: 500, height: 500, x: 0, y: 0, toJSON: () => ({}) };
    });
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockReturnValue(900);
    Element.prototype.scrollIntoView = vi.fn();
    rerender(view);
    const canvas = container.querySelector<HTMLElement>('.shell-column-canvas')!;
    return { canvas, resize: (next: typeof bounds) => { bounds = next; act(() => notify()); }, move: (next: typeof bounds) => { bounds = next; fireEvent.scroll(canvas); } };
  }

  it('reveals a widening visible Column without moving focus or vertical scroll', () => {
    const { canvas, resize } = fixture();
    const draft = screen.getByRole('textbox');
    draft.focus();
    canvas.scrollTop = 20;
    resize({ left: 500, right: 1300, width: 800 });
    expect(canvas.scrollLeft).toBe(400);
    expect(canvas.scrollTop).toBe(20);
    expect(draft).toHaveFocus();
    expect(draft).toHaveValue('unsent');
  });

  it('does not pull back a Column after manual departure', () => {
    const { canvas, move, resize } = fixture();
    move({ left: 1400, right: 1800, width: 400 });
    resize({ left: 1400, right: 2200, width: 800 });
    expect(canvas.scrollLeft).toBe(0);
  });

  it('ignores observations with unchanged width', () => {
    const { canvas, resize } = fixture();
    resize({ left: 500, right: 900, width: 400 });
    expect(canvas.scrollLeft).toBe(0);
  });
});
