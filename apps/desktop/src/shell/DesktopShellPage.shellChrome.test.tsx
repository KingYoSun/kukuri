import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';
import { App } from '@/App';
import {
  closestSection,
  getFloatingActionButton,
  getPrimaryNavigation,
  getWorkspaceTabs,
  openChannelManager,
  openSettingsSection,
  renderAtHash,
  selectTimelineView,
  selectWorkspace,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('mobile nav trigger is footer-only and desktop omits it', async () => {
  const { unmount } = render(<App api={createDesktopMockApi()} />);

  expect(screen.queryByTestId('shell-nav-trigger')).not.toBeInTheDocument();

  unmount();
  setViewportWidth(640);
  render(<App api={createDesktopMockApi()} />);

  expect(await screen.findByTestId('shell-nav-trigger')).toBeInTheDocument();
});

test('floating action button tracks the active section and hides on profile', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  expect(getFloatingActionButton()).toHaveAccessibleName('Publish');
  expect(getFloatingActionButton()).toHaveClass('shell-fab');

  await selectWorkspace(user, 'Live');
  expect(getFloatingActionButton()).toHaveAccessibleName('Start Live');

  await selectWorkspace(user, 'Game');
  expect(getFloatingActionButton()).toHaveAccessibleName('Create Room');

  await selectWorkspace(user, 'Timeline');
  await selectTimelineView(user, 'Bookmarks');
  expect(screen.queryByTestId('shell-fab')).not.toBeInTheDocument();

  await selectWorkspace(user, 'Profile');
  expect(screen.queryByTestId('shell-fab')).not.toBeInTheDocument();
});

test('channel manager opens as a modal from the navigation summary', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  expect(getPrimaryNavigation().querySelector('.shell-nav-accordion-trigger')).toBeNull();
  const channelButton = screen.getByRole('button', { name: 'Private Channels' });
  expect(channelButton).toHaveClass('shell-icon-button');
  expect(channelButton).not.toHaveTextContent('Private Channels');

  const dialog = await openChannelManager(user);
  expect(dialog).toBeInTheDocument();
  expect(dialog).toHaveAccessibleName('Create / Join Private Channel');
  expect(within(dialog).getByText('Create')).toBeInTheDocument();
  expect(within(dialog).getAllByText('Join').length).toBeGreaterThan(0);
  expect(within(dialog).getByText('Channel name')).toBeInTheDocument();
  expect(within(dialog).getByPlaceholderText('Channel name')).toBeInTheDocument();

  await user.click(within(dialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
    ).not.toBeInTheDocument();
  });
});

test('settings hash route opens the drawer and keeps the selected section in sync', async () => {
  const user = userEvent.setup();
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo&settings=discovery');

  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  await waitFor(() => {
    expect(within(drawer).getByTestId('settings-section-discovery')).toHaveAttribute(
      'aria-current',
      'location'
    );
  });

  await user.click(within(drawer).getByTestId('settings-section-connectivity'));

  await waitFor(() => {
    expect(window.location.hash).toContain('settings=connectivity');
  });
});

test('desktop shell defaults to the dark theme and persists it locally', async () => {
  render(<App api={createDesktopMockApi()} />);

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
  });
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('dark');
});

test('desktop shell restores a persisted light theme on boot', async () => {
  window.localStorage.setItem(DESKTOP_THEME_STORAGE_KEY, 'light');

  render(<App api={createDesktopMockApi()} />);

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
  });
});

test('appearance settings deep link updates the document theme and storage immediately', async () => {
  const user = userEvent.setup();
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo&settings=appearance');

  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  await waitFor(() => {
    expect(within(drawer).getByTestId('settings-section-appearance')).toHaveAttribute(
      'aria-current',
      'location'
    );
    expect(document.documentElement).toHaveAttribute('data-theme', 'dark');
  });

  await user.click(within(drawer).getByRole('radio', { name: /Light/i }));

  await waitFor(() => {
    expect(document.documentElement).toHaveAttribute('data-theme', 'light');
  });
  expect(window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY)).toBe('light');
});

test('settings drawer removes redundant section copy and duplicate headings', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const drawer = await openSettingsSection(user, 'appearance');

  expect(within(drawer).queryByText('Current section')).not.toBeInTheDocument();
  expect(
    within(drawer).queryByText('Local light, dark, and language selection.')
  ).not.toBeInTheDocument();
  expect(within(drawer).queryByRole('heading', { name: 'Appearance', level: 3 })).not.toBeInTheDocument();
  expect(within(drawer).getByRole('button', { name: 'Close settings' })).toBeInTheDocument();
});

test('desktop shell can update discovery seeds', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const setDiscoverySeeds = vi.fn(api.setDiscoverySeeds);
  api.setDiscoverySeeds = setDiscoverySeeds;

  render(<App api={api} />);

  const drawer = await openSettingsSection(user, 'discovery');
  const seedEditor = within(drawer).getByPlaceholderText('node_id or node_id@host:port');
  await user.type(seedEditor, 'seed-peer-1');
  await user.click(within(drawer).getByRole('button', { name: 'Save Seeds' }));

  await waitFor(() => {
    expect(setDiscoverySeeds).toHaveBeenCalledWith(['seed-peer-1']);
  });
  expect(within(drawer).getAllByText('seed-peer-1').length).toBeGreaterThan(0);
});

test('desktop shell surfaces docs-assisted topic recovery in diagnostics', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi({ assistPeerIds: ['relay-peer'] })} />);

  const drawer = await openSettingsSection(user, 'discovery');
  await waitFor(() => {
    expect(within(drawer).getByText('Docs Assist Peers')).toBeInTheDocument();
    expect(within(drawer).getAllByText('relay-peer').length).toBeGreaterThan(0);
  });

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:relay');
  await user.click(screen.getByRole('button', { name: 'Add' }));

  await waitFor(() => {
    const relayTopic = screen.getByRole('button', { name: 'kukuri:topic:relay' }).closest('li');
    expect(relayTopic).not.toBeNull();
    expect(relayTopic).toHaveTextContent('recovering / peers: 0');
    expect(relayTopic).not.toHaveTextContent('relay-assisted sync available via 1 peer(s)');
  });

  await user.click(within(drawer).getByTestId('settings-section-connectivity'));
  const relayHeading = await within(drawer).findByRole('heading', { name: 'kukuri:topic:relay' });
  const relaySection = closestSection(relayHeading);
  expect(
    within(relaySection).getByText(
      'docs-assisted recovery is in progress via 1 peer(s); live topic delivery is unavailable'
    )
  ).toBeInTheDocument();
});

test('desktop shell renders diagnostics error reasons', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        globalLastError: 'failed to import peer ticket: invalid endpoint id',
        topicLastError: 'timed out waiting for gossip topic join',
      })}
    />
  );

  const drawer = await openSettingsSection(user, 'connectivity');
  await waitFor(() => {
    expect(
      within(drawer).getByText('failed to import peer ticket: invalid endpoint id')
    ).toBeInTheDocument();
  });

  const topicHeading = await within(drawer).findByRole('heading', { name: 'kukuri:topic:demo' });
  const topicSection = closestSection(topicHeading);
  expect(within(topicSection).getByText('timed out waiting for gossip topic join')).toBeInTheDocument();
});

test('desktop shell primary nav jumps focus and settings drawer restores trigger focus on escape', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const gameNav = within(getWorkspaceTabs()).getByRole('tab', { name: 'Game' });
  await user.click(gameNav);

  const gameSection = screen.getByText('Metaverse Rooms').closest('.shell-section');
  if (!(gameSection instanceof HTMLElement)) {
    throw new Error('game section not found');
  }

  await waitFor(() => {
    expect(gameNav).toHaveAttribute('aria-selected', 'true');
    expect(gameSection).toHaveFocus();
  });

  const settingsTrigger = screen.getByTestId('shell-settings-trigger');
  expect(settingsTrigger.querySelector('.lucide-settings')).toBeTruthy();
  expect(settingsTrigger.querySelector('.lucide-settings-2')).toBeFalsy();
  await user.click(settingsTrigger);
  await screen.findByRole('dialog', { name: 'Settings' });

  fireEvent.keyDown(window, { key: 'Escape' });

  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
  });
  expect(settingsTrigger).toHaveFocus();
});

