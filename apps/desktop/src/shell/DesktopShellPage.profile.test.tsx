import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  expectActiveTopic,
  getSocialConnectionsTabs,
  openChannelManager,
  openSettingsDrawer,
  openSettingsSection,
  publishPost,
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

test('profile overview aggregates public posts across topics and excludes private channel posts', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'demo public post');
  await waitFor(() => {
    expect(screen.getByText('demo public post')).toBeInTheDocument();
  });

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(window.location.hash).toMatch(
      /^#\/timeline\?topic=kukuri%3Atopic%3Ademo&channel=channel-\d+$/
    );
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
    ).not.toBeInTheDocument();
  });
  await publishPost(user, 'demo private post');
  await waitFor(() => {
    expect(screen.getByText('demo private post')).toBeInTheDocument();
  });

  await selectWorkspace(user, 'Profile');
  expect(screen.getByText('demo public post')).toBeInTheDocument();
  expect(screen.queryByText('demo private post')).not.toBeInTheDocument();
  expect(screen.getAllByText('kukuri:topic:demo').length).toBeGreaterThan(0);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));
  await waitFor(() => {
    expectActiveTopic('kukuri:topic:second');
  });

  await selectWorkspace(user, 'Timeline');
  await publishPost(user, 'second public post');
  await waitFor(() => {
    expect(screen.getByText('second public post')).toBeInTheDocument();
  });

  await selectWorkspace(user, 'Profile');
  expect(screen.getByText('demo public post')).toBeInTheDocument();
  expect(screen.getByText('second public post')).toBeInTheDocument();
  expect(screen.queryByText('demo private post')).not.toBeInTheDocument();
  const profileSection = screen.getByText('second public post').closest('.shell-section');
  if (!(profileSection instanceof HTMLElement)) {
    throw new Error('profile section not found');
  }
  expect(within(profileSection).queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();
  expect(within(profileSection).getAllByRole('button', { name: 'Open original topic' }).length).toBe(2);
});

test('profile overview connection count buttons open the requested connections tab', async () => {
  const followedPubkey = 'b'.repeat(64);
  const mutedPubkey = 'c'.repeat(64);
  const user = userEvent.setup();

  render(
    <App
      api={createDesktopMockApi({
        authorSocialViews: {
          [followedPubkey]: {
            name: 'bob',
            followed_by: true,
          },
          [mutedPubkey]: {
            name: 'carol',
            muted: true,
          },
        },
      })}
    />
  );

  await selectWorkspace(user, 'Profile');
  await user.click(screen.getByRole('button', { name: '1 Followers' }));

  await waitFor(() => {
    expect(within(getSocialConnectionsTabs()).getByRole('tab', { name: 'Followed' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  expect(
    screen.queryByText('Followed shows only followers already observed on this device.')
  ).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Back to profile' }));
  await user.click(screen.getByRole('button', { name: '1 Muted' }));

  await waitFor(() => {
    expect(within(getSocialConnectionsTabs()).getByRole('tab', { name: 'Muted' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
});

test('author detail shows profile topic posts and can open an untracked origin topic', async () => {
  const authorPubkey = 'b'.repeat(64);
  const user = userEvent.setup();

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'post-author-demo',
              envelope_id: 'envelope-author-demo',
              author_pubkey: authorPubkey,
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'post from demo topic',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-author-demo',
              audience_label: 'Public',
            },
          ],
          'kukuri:topic:relay': [
            {
              object_id: 'post-author-relay',
              envelope_id: 'envelope-author-relay',
              author_pubkey: authorPubkey,
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'post from relay topic',
              content_status: 'Available',
              attachments: [],
              created_at: 2,
              reply_to: null,
              root_id: 'post-author-relay',
              audience_label: 'Public',
            },
          ],
        },
        authorSocialViews: {
          [authorPubkey]: {
            name: 'bob',
            about: 'author detail profile feed',
          },
        },
      })}
    />
  );

  await user.click(await screen.findByRole('button', { name: 'bob' }));

  const authorPane = await screen.findByRole('complementary', { name: 'Author' });
  expect(within(authorPane).getByText('post from demo topic')).toBeInTheDocument();
  expect(within(authorPane).getByText('post from relay topic')).toBeInTheDocument();
  expect(within(authorPane).getByText('kukuri:topic:relay')).toBeInTheDocument();
  expect(within(authorPane).queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();

  await user.click(within(authorPane).getAllByRole('button', { name: 'Open original topic' })[0]);

  await waitFor(() => {
    expectActiveTopic('kukuri:topic:relay');
    expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();
  });
  expect(screen.getByText('post from relay topic')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'kukuri:topic:relay' })).toBeInTheDocument();
});

test('local profile editor saves profile draft from primary navigation and settings stays diagnostics-only', async () => {
  const api = createDesktopMockApi();
  const user = userEvent.setup();

  render(<App api={api} />);

  await selectWorkspace(user, 'Profile');
  await user.click(screen.getByRole('button', { name: 'Edit Profile' }));
  const profileSection = screen.getByPlaceholderText('Visible label').closest('.shell-section');
  if (!(profileSection instanceof HTMLElement)) {
    throw new Error('profile section not found');
  }

  const displayNameInput = within(profileSection).getByPlaceholderText('Visible label');
  await user.type(displayNameInput, 'Local Author');
  await user.click(within(profileSection).getByRole('button', { name: 'Save Profile' }));

  await waitFor(() => {
    expect(screen.getByText('Local Author')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Edit Profile' })).toBeInTheDocument();
    expect(window.location.hash).toBe('#/profile?topic=kukuri%3Atopic%3Ademo');
  });

  const drawer = await openSettingsDrawer(user);
  expect(within(drawer).queryByTestId('settings-section-profile')).not.toBeInTheDocument();
});

test('keeps local peer ticket visible when profile loading fails', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi({ myProfileError: 'profile load failed' })} />);

  const drawer = await openSettingsSection(user, 'connectivity');
  await waitFor(() => {
    expect(within(drawer).getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  });
});
