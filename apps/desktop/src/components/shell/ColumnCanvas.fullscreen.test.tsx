import { act, render } from '@testing-library/react';
import { afterEach, expect, it, vi } from 'vitest';
import { ColumnCanvas } from './ColumnCanvas';

afterEach(() => {
  Reflect.deleteProperty(document, 'fullscreenElement');
  vi.unstubAllGlobals();
});

it('keeps the fullscreen owner visible despite late non-intersection and resumes normal observation on exit', () => {
  let notify: IntersectionObserverCallback;
  const disconnect = vi.fn();
  vi.stubGlobal('IntersectionObserver', class {
    constructor(callback: IntersectionObserverCallback) { notify = callback; }
    observe = vi.fn();
    disconnect = disconnect;
  });
  Element.prototype.scrollIntoView = vi.fn();
  const publish = vi.fn();
  const { container, unmount } = render(
    <ColumnCanvas activeColumnId='room' columnIds={['room', 'stream']} onActivateColumn={vi.fn()} onVisibleColumnIdsChange={publish}>
      <section data-column-id='room'><canvas /></section>
      <section data-column-id='stream' />
    </ColumnCanvas>
  );
  const room = container.querySelector('[data-column-id="room"]')!;
  const stream = container.querySelector('[data-column-id="stream"]')!;
  const intersect = (target: Element, visible: boolean) => act(() => notify([
    { target, isIntersecting: visible, intersectionRatio: visible ? 1 : 0 } as IntersectionObserverEntry,
  ], {} as IntersectionObserver));
  const fullscreen = (element: Element | null) => act(() => {
    Object.defineProperty(document, 'fullscreenElement', { configurable: true, value: element });
    document.dispatchEvent(new Event('fullscreenchange'));
  });
  intersect(room, true);
  intersect(stream, true);
  expect(publish).toHaveBeenLastCalledWith(['room', 'stream']);
  fullscreen(room);
  expect(publish).toHaveBeenLastCalledWith(['room']);
  intersect(room, false);
  intersect(stream, true);
  expect(publish).toHaveBeenLastCalledWith(['room']);
  fullscreen(null);
  intersect(room, true);
  expect(publish).toHaveBeenLastCalledWith(['room', 'stream']);
  intersect(room, false);
  expect(publish).toHaveBeenLastCalledWith(['stream']);
  // A nested media fullscreen belongs to its Column too.
  fullscreen(room.querySelector('canvas'));
  expect(publish).toHaveBeenLastCalledWith(['room']);
  unmount();
  publish.mockClear();
  fullscreen(null);
  expect(disconnect).toHaveBeenCalledOnce();
  expect(publish).not.toHaveBeenCalled();
});
