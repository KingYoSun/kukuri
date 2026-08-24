import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, expect, test, vi } from 'vitest';

import { ColumnSurface } from './ColumnSurface';

let fullscreenElement: Element | null = null;

afterEach(() => {
  fullscreenElement = null;
  vi.restoreAllMocks();
});

test('a fullscreen-capable column enters and exits fullscreen from its menu', async () => {
  const user = userEvent.setup();
  Object.defineProperty(document, 'fullscreenElement', {
    configurable: true,
    get: () => fullscreenElement,
  });
  Object.defineProperty(document, 'fullscreenEnabled', {
    configurable: true,
    value: true,
  });
  const requestFullscreen = vi.fn<() => Promise<void>>();
  Object.defineProperty(HTMLElement.prototype, 'requestFullscreen', {
    configurable: true,
    value: requestFullscreen,
  });
  Object.defineProperty(document, 'exitFullscreen', {
    configurable: true,
    value: vi.fn(async () => {
      fullscreenElement = null;
      document.dispatchEvent(new Event('fullscreenchange'));
    }),
  });

  render(
    <div className='shell-phase1'>
      <ColumnSurface
        active
        columnId='stream-1'
        fullscreenable
        pinned
        position={1}
        scopeLabel='Room'
        span={2}
        title='Stream'
        total={1}
      >
        Stream body
      </ColumnSurface>
    </div>
  );

  const column = screen.getByRole('region', { name: /Stream Column/ });
  requestFullscreen.mockImplementation(async () => {
    fullscreenElement = column;
    document.dispatchEvent(new Event('fullscreenchange'));
  });
  await user.click(screen.getByRole('button', { name: 'Open Stream menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Enter Stream fullscreen' }));
  await waitFor(() => expect(document.fullscreenElement).toBe(column));

  await user.click(screen.getByRole('button', { name: 'Open Stream menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Exit Stream fullscreen' }));
  await waitFor(() => expect(document.fullscreenElement).toBeNull());
});

test('an unavailable fullscreen API reports a visible assistive failure', async () => {
  const user = userEvent.setup();
  Object.defineProperty(document, 'fullscreenEnabled', {
    configurable: true,
    value: false,
  });

  render(
    <div className='shell-phase1'>
      <ColumnSurface
        active
        columnId='stream-1'
        fullscreenable
        pinned
        position={1}
        scopeLabel='Room'
        span={2}
        title='Stream'
        total={1}
      >
        Stream body
      </ColumnSurface>
    </div>
  );

  await user.click(screen.getByRole('button', { name: 'Open Stream menu' }));
  await user.click(screen.getByRole('menuitem', { name: 'Enter Stream fullscreen' }));

  expect(await screen.findByText('Could not change Stream fullscreen mode.')).toBeVisible();
});
