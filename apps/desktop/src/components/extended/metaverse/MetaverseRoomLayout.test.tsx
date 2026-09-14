import { createRef, useEffect } from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ColumnFullscreenContext } from '@/shell/ColumnPresentationContext';
import { MetaverseRoomLayout } from './MetaverseRoomLayout';

it('preserves scene, form and draft through fullscreen tools and return without repeating mount effects', () => {
  const mount = vi.fn();
  const unmount = vi.fn();
  function Session() {
    useEffect(() => { mount(); return unmount; }, []);
    return <div className='metaverse-room-view'><input aria-label='Chat draft' /></div>;
  }
  const panelRef = createRef<HTMLDivElement>();
  const view = (fullscreen: boolean, admitted = true) => (
    <ColumnFullscreenContext.Provider value={fullscreen}>
      <MetaverseRoomLayout admitted={admitted} panelRef={panelRef}
        before={<input aria-label='Dome name' defaultValue='Original' />}
        after={<button>Hosting action</button>}>
        <Session />
      </MetaverseRoomLayout>
    </ColumnFullscreenContext.Provider>
  );
  const { rerender } = render(view(false));
  const draft = screen.getByRole('textbox', { name: 'Chat draft' });
  const name = screen.getByRole('textbox', { name: 'Dome name' });
  fireEvent.change(draft, { target: { value: 'Unsent 1023' } });
  fireEvent.change(name, { target: { value: 'Edited name' } });
  rerender(view(true));
  expect(name).not.toBeVisible();
  expect(screen.queryByRole('button', { name: 'Hosting action' })).toBeNull();
  fireEvent.click(screen.getByRole('button', { name: 'Dome tools' }));
  expect(name).toBeVisible();
  expect(name).toHaveValue('Edited name');
  const close = screen.getAllByRole('button', { name: 'Close Dome tools' });
  fireEvent.click(close[1]);
  expect(screen.getByRole('button', { name: 'Dome tools' })).toHaveFocus();
  expect(name).not.toBeVisible();
  rerender(view(false));
  expect(name).toBeVisible();
  expect(screen.getByRole('textbox', { name: 'Chat draft' })).toBe(draft);
  expect(draft).toHaveValue('Unsent 1023');
  expect(mount).toHaveBeenCalledOnce();
  expect(unmount).not.toHaveBeenCalled();
  rerender(view(true, false));
  expect(name).toBeVisible();
  expect(screen.getByRole('button', { name: 'Hosting action' })).toBeVisible();
  expect(screen.queryByRole('button', { name: 'Dome tools' })).toBeNull();
});
