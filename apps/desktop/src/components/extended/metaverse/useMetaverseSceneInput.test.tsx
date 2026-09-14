import { StrictMode, useRef } from 'react';
import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useMetaverseSceneInput } from './useMetaverseSceneInput';

function Harness({ eligible = true, room = 'room-a' }: { eligible?: boolean; room?: string }) {
  const stageRef = useRef<HTMLDivElement>(null);
  const input = useMetaverseSceneInput(stageRef, eligible, room);
  return <><div ref={stageRef} tabIndex={0}><canvas /><output>{input.mode}</output></div>
    <button onClick={input.start}>Start</button><button onClick={input.release}>Release</button><input aria-label='Outside' /></>;
}
let locked: Element | null = null;
let request: ReturnType<typeof vi.fn>;
let exit: ReturnType<typeof vi.fn>;
beforeEach(() => {
  locked = null;
  request = vi.fn();
  exit = vi.fn(() => { locked = null; document.dispatchEvent(new Event('pointerlockchange')); });
  vi.spyOn(document, 'hasFocus').mockReturnValue(true);
  vi.stubGlobal('requestAnimationFrame', (fn: FrameRequestCallback) => setTimeout(fn, 0));
  Object.defineProperty(document, 'pointerLockElement', { configurable: true, get: () => locked });
  Object.defineProperty(document, 'exitPointerLock', { configurable: true, value: exit });
  Object.defineProperty(HTMLCanvasElement.prototype, 'requestPointerLock', { configurable: true, value: request });
});
afterEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); delete (HTMLCanvasElement.prototype as Partial<HTMLCanvasElement>).requestPointerLock; });
function confirm(canvas: Element) { act(() => { locked = canvas; document.dispatchEvent(new Event('pointerlockchange')); }); }

test('waits for actual ownership and releases on blur without automatically recapturing', () => {
  const view = render(<StrictMode><Harness /></StrictMode>);
  expect(screen.getByText('idle')).toBeInTheDocument();
  fireEvent.click(screen.getByText('Start'));
  expect(screen.getByText('requesting')).toBeInTheDocument();
  confirm(view.container.querySelector('canvas')!);
  expect(screen.getByText('locked')).toBeInTheDocument();
  fireEvent.blur(window);
  expect(exit).toHaveBeenCalledTimes(1);
  expect(screen.getByText('idle')).toBeInTheDocument();
  fireEvent.focus(window);
  expect(request).toHaveBeenCalledTimes(1);
});
test('runtime suspension releases ownership and clears a pending request', () => {
  const view = render(<Harness />);
  fireEvent.click(screen.getByText('Start'));
  view.rerender(<Harness eligible={false} />);
  confirm(view.container.querySelector('canvas')!);
  expect(locked).toBeNull();
  expect(screen.getByText('idle')).toBeInTheDocument();
  fireEvent.click(screen.getByText('Start'));
  expect(request).toHaveBeenCalledTimes(1);
});
test('a pending request cannot capture after opening UI or switching rooms', async () => {
  let resolve!: () => void;
  request.mockImplementation(() => new Promise<void>(done => { resolve = done; }));
  const view = render(<Harness />);
  fireEvent.click(screen.getByText('Start'));
  fireEvent.click(screen.getByText('Release'));
  view.rerender(<Harness room='room-b' />);
  await act(async () => { locked = view.container.querySelector('canvas'); resolve(); });
  expect(locked).toBeNull();
  expect(screen.getByText('idle')).toBeInTheDocument();
});
test('unsupported and rejected capture offers a retry without a retry loop', async () => {
  request.mockRejectedValue(new Error('denied'));
  render(<Harness />);
  fireEvent.click(screen.getByText('Start'));
  expect(await screen.findByText('unavailable')).toBeInTheDocument();
  expect(request).toHaveBeenCalledTimes(1);
  fireEvent.click(screen.getByText('Start'));
  expect(await screen.findByText('unavailable')).toBeInTheDocument();
  expect(request).toHaveBeenCalledTimes(2);
});
test('does not steal or release another canvas lock', () => {
  locked = document.createElement('canvas');
  const view = render(<Harness />);
  fireEvent.click(screen.getByText('Start'));
  fireEvent.click(screen.getByText('Release'));
  view.unmount();
  expect(request).not.toHaveBeenCalled();
  expect(exit).not.toHaveBeenCalled();
});
test('moving focus into another UI releases the canvas', () => {
  const view = render(<Harness />);
  fireEvent.click(screen.getByText('Start'));
  confirm(view.container.querySelector('canvas')!);
  act(() => screen.getByLabelText('Outside').focus());
  expect(locked).toBeNull();
});
