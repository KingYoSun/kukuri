import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { App } from '@/App';
import type { TimelineView } from '@/lib/api';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { createDeferred, selectWorkspace, setViewportWidth } from './DesktopShellPage.testHelpers';

beforeEach(() => {
  setViewportWidth(1280);
  window.history.replaceState(null, '', '/');
});

test('a fast refresh shows feedback for at least one second without delaying data', async () => {
  const api = createDesktopMockApi();
  render(<App api={api} />);
  const profile = await screen.findByRole('region', { name: /^Profile Column,/ });
  const refresh = await within(profile).findByRole('button', { name: 'Refresh profile' }, { timeout: 2000 });
  await waitFor(() => expect(refresh).toHaveAttribute('aria-busy', 'false'));
  await api.setMyProfile({ display_name: 'Updated immediately', name: 'updated' });
  vi.useFakeTimers();
  await act(async () => { fireEvent.click(refresh); });
  expect(within(profile).getByRole('heading', { name: 'Updated immediately' })).toBeInTheDocument();
  expect(refresh).toHaveAttribute('aria-busy', 'true');
  await act(async () => vi.advanceTimersByTimeAsync(999));
  expect(refresh).toHaveAttribute('aria-busy', 'true');
  await act(async () => vi.advanceTimersByTimeAsync(1));
  expect(refresh).toHaveAttribute('aria-busy', 'false');
  vi.useRealTimers();
});

test('selecting columns does not reload the open profile', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  render(<App api={api} />);
  const profile = await screen.findByRole('region', { name: /^Profile Column,/ });
  await within(profile).findByText('No public posts published yet.');
  const read = vi.spyOn(api, 'listProfileTimeline');
  for (const name of [/^Profile Column,/, /^Explore Column,/, /^Timeline Column,/]) {
    await user.click(screen.getByRole('region', { name }));
  }
  expect(read).not.toHaveBeenCalled();
});

test('manual refresh retains an empty feed and focus while preventing duplicate clicks', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  render(<App api={api} />);
  const profile = await screen.findByRole('region', { name: /^Profile Column,/ });
  await within(profile).findByText('No public posts published yet.');
  const active = document.querySelector('[data-column-id][aria-current="true"]');
  const pending = createDeferred<TimelineView>();
  const read = vi.spyOn(api, 'listProfileTimeline').mockReturnValue(pending.promise);
  const refresh = await within(profile).findByRole('button', { name: 'Refresh profile' }, { timeout: 2000 });
  await user.click(refresh);
  await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
  expect(refresh).toHaveAttribute('aria-busy', 'true');
  expect(refresh).toHaveAccessibleName('Refreshing profile');
  expect(refresh).toHaveFocus();
  expect(within(profile).queryByText('Loading profile…')).not.toBeInTheDocument();
  expect(within(profile).getByText('No public posts published yet.')).toBeInTheDocument();
  await user.click(refresh);
  await user.keyboard('{Enter}');
  expect(read).toHaveBeenCalledTimes(1);
  expect(document.querySelector('[data-column-id][aria-current="true"]')).toBe(active);
  await act(async () => pending.resolve({ items: [], next_cursor: null }));
  await waitFor(() => expect(refresh).toHaveAttribute('aria-busy', 'false'));
  expect(refresh).toHaveFocus();
});

test('reopening a profile reuses confirmed content and refreshes once', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  render(<App api={api} />);
  const profile = await screen.findByRole('region', { name: /^Profile Column,/ });
  await within(profile).findByText('No public posts published yet.');
  await user.click(within(profile).getByRole('button', { name: 'Close Profile' }));
  const pending = createDeferred<TimelineView>();
  const read = vi.spyOn(api, 'listProfileTimeline').mockReturnValue(pending.promise);
  await selectWorkspace(user, 'Profile');
  const reopened = screen.getByRole('region', { name: /^Profile Column,/ });
  await waitFor(() => expect(read).toHaveBeenCalledTimes(1));
  expect(within(reopened).getByText('No public posts published yet.')).toBeInTheDocument();
  expect(within(reopened).queryByText('Loading profile…')).not.toBeInTheDocument();
  await act(async () => pending.resolve({ items: [], next_cursor: null }));
});
