import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  closestSection,
  openSettingsSection,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { DesktopApi } from '@/lib/api';
import { REFRESH_INTERVAL_MS } from '@/shell/store';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('community node panel keeps the auto-approve node active on the current session', async () => {
  const api = createDesktopMockApi();
  const user = userEvent.setup();

  render(<App api={api} />);

  const drawer = await openSettingsSection(user, 'community-node');
  const nodeHeading = await within(drawer).findByText('https://api.kukuri.app', { selector: 'h4' });
  const blockElement = closestSection(nodeHeading);
  expect(
    within(blockElement).getByRole('checkbox', {
      name: 'Auto-approve consent for this node',
    })
  ).toBeChecked();

  await waitFor(() => {
    expect(within(blockElement).getAllByText('https://api.kukuri.app').length).toBeGreaterThan(0);
    expect(within(blockElement).getByText('active on current session')).toBeInTheDocument();
    expect(
      within(blockElement).getByText('connectivity urls active on current session')
    ).toBeInTheDocument();
  });
});

test('community-node failure marks the global trigger without warning an unaffected Timeline Column', async () => {
  const user = userEvent.setup();
  const baseApi = createDesktopMockApi();
  const baseStatuses = await baseApi.getCommunityNodeStatuses();
  const api: DesktopApi = {
    ...baseApi,
    async getCommunityNodeStatuses() {
      return baseStatuses.map((status) => ({
        ...status,
        last_error: 'community node timeout',
        session_phase: 'retrying',
      }));
    },
  };

  render(<App api={api} />);

  const trigger = await screen.findByTestId('control-center-trigger');
  await waitFor(() => {
    expect(trigger).toHaveAccessibleName('Open Control Center · Community node needs attention');
  });
  const timelineColumn = screen.getByRole('region', { name: /^Timeline Column,/ });
  expect(
    within(timelineColumn).queryByTestId('community-node-unavailable-notice')
  ).not.toBeInTheDocument();

  await user.click(trigger);
  const controlCenter = screen.getByRole('complementary', { name: 'Control Center' });
  await user.click(
    within(controlCenter).getByRole('button', { name: 'Open Community Node Settings' })
  );

  const drawer = await screen.findByRole('dialog', { name: 'Settings' });
  expect(within(drawer).getByTestId('settings-section-community-node')).toHaveAttribute(
    'aria-current',
    'location'
  );
  expect(within(drawer).getByRole('heading', { name: 'Community Node' })).toBeInTheDocument();
  expect(window.location.hash).toContain('settings=community-node');
});

test('global community-node status clears after the node recovers', async () => {
  let recovered = false;
  const baseApi = createDesktopMockApi();
  const baseStatuses = await baseApi.getCommunityNodeStatuses();
  const api: DesktopApi = {
    ...baseApi,
    async getCommunityNodeStatuses() {
      return baseStatuses.map((status) => ({
        ...status,
        last_error: recovered ? null : 'community node timeout',
        session_phase: recovered ? 'ready' : 'retrying',
      }));
    },
  };

  render(<App api={api} />);

  const trigger = await screen.findByTestId('control-center-trigger');
  await waitFor(() => {
    expect(trigger).toHaveAccessibleName('Open Control Center · Community node needs attention');
  });

  recovered = true;
  await new Promise((resolve) => window.setTimeout(resolve, REFRESH_INTERVAL_MS + 300));

  await waitFor(() => {
    expect(trigger).toHaveAccessibleName('Open Control Center · Connected');
  });
});

test('Explore owns the affected-column notice and hides the healthy primary node selector', async () => {
  const healthyApi = createDesktopMockApi();
  const healthy = render(<App api={healthyApi} />);
  const healthyUser = userEvent.setup();

  await healthyUser.click(await screen.findByTestId('control-center-trigger'));
  const healthyControlCenter = screen.getByRole('complementary', { name: 'Control Center' });
  await healthyUser.click(within(healthyControlCenter).getByRole('button', { name: 'Add Explore Column' }));

  const explore = await screen.findByTestId('community-index-explore');
  expect(within(explore).queryByText('Community node')).not.toBeInTheDocument();
  expect(within(explore).queryByRole('combobox')).not.toBeInTheDocument();
  expect(screen.queryByTestId('community-index-topic')).not.toBeInTheDocument();

  healthy.unmount();

  const baseApi = createDesktopMockApi();
  const baseStatuses = await baseApi.getCommunityNodeStatuses();
  const failingApi: DesktopApi = {
    ...baseApi,
    async getCommunityNodeStatuses() {
      return baseStatuses.map((status) => ({
        ...status,
        last_error: 'community node timeout',
        session_phase: 'retrying',
      }));
    },
  };
  const failingUser = userEvent.setup();
  render(<App api={failingApi} />);
  await failingUser.click(await screen.findByTestId('control-center-trigger'));
  const failingControlCenter = screen.getByRole('complementary', { name: 'Control Center' });
  await failingUser.click(within(failingControlCenter).getByRole('button', { name: 'Add Explore Column' }));

  const notice = await screen.findByTestId('community-node-unavailable-notice');
  const exploreColumn = notice.closest('[data-column-id]');
  expect(exploreColumn).toHaveAccessibleName(/^Explore Column/);
});

test('timeline keeps the last successful workspace state when joined channels refresh fails', async () => {
  let failNextJoinedChannelsRefresh = false;
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-refresh-joined-channels',
          envelope_id: 'envelope-refresh-joined-channels',
          author_pubkey: 'a'.repeat(64),
          author_name: 'alice',
          author_display_name: null,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'joined channel refresh fallback',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-refresh-joined-channels',
          audience_label: 'Public',
        },
      ],
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async listJoinedPrivateChannels(topic) {
      if (failNextJoinedChannelsRefresh) {
        failNextJoinedChannelsRefresh = false;
        throw new Error('temporary joined channel failure');
      }
      return baseApi.listJoinedPrivateChannels(topic);
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('joined channel refresh fallback')).toBeInTheDocument();

  failNextJoinedChannelsRefresh = true;
  await new Promise((resolve) => window.setTimeout(resolve, REFRESH_INTERVAL_MS + 300));

  await waitFor(() => {
    expect(screen.getByText('joined channel refresh fallback')).toBeInTheDocument();
    expect(screen.getByText('temporary joined channel failure')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: /Timeline Column/ })).toHaveTextContent('demo');
  });
});

test('timeline keeps the last successful workspace state when community-node status refresh fails', async () => {
  let failNextCommunityNodeRefresh = false;
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-refresh-community-node-status',
          envelope_id: 'envelope-refresh-community-node-status',
          author_pubkey: 'a'.repeat(64),
          author_name: 'alice',
          author_display_name: null,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'community node refresh fallback',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-refresh-community-node-status',
          audience_label: 'Public',
        },
      ],
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async getCommunityNodeStatuses() {
      if (failNextCommunityNodeRefresh) {
        failNextCommunityNodeRefresh = false;
        throw new Error('temporary community node status failure');
      }
      return baseApi.getCommunityNodeStatuses();
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('community node refresh fallback')).toBeInTheDocument();

  failNextCommunityNodeRefresh = true;
  await new Promise((resolve) => window.setTimeout(resolve, REFRESH_INTERVAL_MS + 300));

  await waitFor(() => {
    expect(screen.getByText('community node refresh fallback')).toBeInTheDocument();
    expect(
      screen.queryByText('temporary community node status failure')
    ).not.toBeInTheDocument();
    expect(screen.getByRole('region', { name: /Timeline Column/ })).toHaveTextContent('demo');
  });
});

