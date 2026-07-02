import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  expectActiveTopic,
  openChannelManager,
  publishPost,
  renderAtHash,
  selectTimelineView,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('bookmark page route closes detail context and normalizes timeline-specific params', async () => {
  renderAtHash(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&timelineView=bookmarks&channel=channel-1&context=thread&threadId=post-thread-open',
    createDesktopMockApi({
      seedPosts: {
        'kukuri:topic:demo': [
          {
            object_id: 'post-thread-open',
            envelope_id: 'envelope-thread-open',
            author_pubkey: 'b'.repeat(64),
            author_name: 'bob',
            author_display_name: null,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            object_kind: 'post',
            content: 'thread should close',
            content_status: 'Available',
            attachments: [],
            created_at: 1,
            reply_to: null,
            root_id: 'post-thread-open',
            channel_id: null,
            audience_label: 'Public',
          },
        ],
      },
    })
  );

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&timelineView=bookmarks');
  });
  expect(screen.queryByRole('complementary', { name: 'Thread' })).not.toBeInTheDocument();
  expect(screen.getByText('No bookmarked posts yet.')).toBeInTheDocument();
});

test('bookmarking from the timeline syncs with the bookmark page and remove updates both views', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'bookmark-me',
              envelope_id: 'envelope-bookmark-me',
              author_pubkey: 'a'.repeat(64),
              author_name: 'alice',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'save this post',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'bookmark-me',
              audience_label: 'Public',
            },
          ],
        },
      })}
    />
  );

  const timelinePost = await screen.findByText('save this post');
  const timelineCard = timelinePost.closest('article');
  if (!(timelineCard instanceof HTMLElement)) {
    throw new Error('timeline card not found');
  }

  await user.click(within(timelineCard).getByRole('button', { name: 'Bookmark' }));
  await waitFor(() => {
    expect(within(timelineCard).getByRole('button', { name: 'Remove bookmark' })).toBeInTheDocument();
  });

  await selectTimelineView(user, 'Bookmarks');
  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&timelineView=bookmarks');
  });
  expect(await screen.findByText('save this post')).toBeInTheDocument();

  const bookmarkedCard = screen.getByText('save this post').closest('article');
  if (!(bookmarkedCard instanceof HTMLElement)) {
    throw new Error('bookmarked card not found');
  }
  await user.click(within(bookmarkedCard).getByRole('button', { name: 'Remove bookmark' }));

  await waitFor(() => {
    expect(screen.getByText('No bookmarked posts yet.')).toBeInTheDocument();
  });

  await selectTimelineView(user, 'Feed');
  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
  const restoredTimelinePost = await screen.findByText('save this post');
  const restoredTimelineCard = restoredTimelinePost.closest('article');
  if (!(restoredTimelineCard instanceof HTMLElement)) {
    throw new Error('restored timeline card not found');
  }
  expect(within(restoredTimelineCard).getByRole('button', { name: 'Bookmark' })).toBeInTheDocument();
});

test('topic and private channel selection sync into the hash route', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Asecond');
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1'
    );
  });
});

test('tracked topics show public and channel scope separately in the sidebar', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

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
    expect(
      screen.getByRole('button', { name: 'Open core channel settings' })
    ).toBeInTheDocument();
  });

  const topicItem = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
  if (!(topicItem instanceof HTMLElement)) {
    throw new Error('active topic item not found');
  }

  expect(within(topicItem).getByText('Channels')).toBeInTheDocument();
  const publicButton = within(topicItem).getByText('Public').closest('button');
  const channelButton = within(topicItem).getByText('core').closest('button');
  if (!(publicButton instanceof HTMLButtonElement)) {
    throw new Error('public scope button not found');
  }
  if (!(channelButton instanceof HTMLButtonElement)) {
    throw new Error('channel scope button not found');
  }

  await waitFor(() => {
    expect(publicButton).toHaveAttribute('aria-pressed', 'false');
    expect(channelButton).toHaveAttribute('aria-pressed', 'true');
  });

  await user.click(publicButton);

  await waitFor(() => {
    expect(publicButton).toHaveAttribute('aria-pressed', 'true');
    expect(channelButton).toHaveAttribute('aria-pressed', 'false');
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
});

test('sidebar can reselect the same private channel after switching back to public', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(window.location.hash).toMatch(
      /^#\/timeline\?topic=kukuri%3Atopic%3Ademo&channel=channel-\d+$/
    );
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  const topicItem = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
  if (!(topicItem instanceof HTMLElement)) {
    throw new Error('active topic item not found');
  }

  const publicButton = within(topicItem).getByText('Public').closest('button');
  const channelButton = within(topicItem).getByText('core').closest('button');
  if (!(publicButton instanceof HTMLButtonElement) || !(channelButton instanceof HTMLButtonElement)) {
    throw new Error('scope buttons not found');
  }

  await user.click(publicButton);
  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });

  await user.click(channelButton);
  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(channelButton).toHaveAttribute('aria-pressed', 'true');
  });
});

test('sidebar can switch from one topic public scope to another topic private channel scope', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'second-core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Asecond&channel=channel-1');
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  await waitFor(() => {
    expectActiveTopic('kukuri:topic:demo');
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));
  const secondTopicItem = screen.getByRole('button', { name: 'kukuri:topic:second' }).closest('li');
  if (!(secondTopicItem instanceof HTMLElement)) {
    throw new Error('second topic item not found');
  }

  const secondChannelButton = within(secondTopicItem).getByText('second-core').closest('button');
  if (!(secondChannelButton instanceof HTMLButtonElement)) {
    throw new Error('second topic channel button not found');
  }

  await user.click(secondChannelButton);
  await waitFor(() => {
    expectActiveTopic('kukuri:topic:second');
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Asecond&channel=channel-1');
    expect(secondChannelButton).toHaveAttribute('aria-pressed', 'true');
  });
});

test('desktop shell can track multiple topics at once', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  expect(screen.getByRole('button', { name: 'kukuri:topic:second' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  await publishPost(user, 'demo post');
  await waitFor(() => {
    expect(screen.getByText('demo post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));
  await publishPost(user, 'second post');
  await waitFor(() => {
    expect(screen.getByText('second post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  const demoTopic = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
  expect(demoTopic).not.toBeNull();
  expect(screen.getByText('demo post')).toBeInTheDocument();
  expect(demoTopic).toHaveTextContent(/\/ peers: \d/);
  expect(demoTopic).not.toHaveTextContent('expected:');
  expect(demoTopic).not.toHaveTextContent('Connected to all configured peers for this topic');
});

test('removing the active topic falls back to the remaining tracked topic', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));

  await waitFor(() => {
    expectActiveTopic('kukuri:topic:second');
  });

  await user.click(screen.getByRole('button', { name: 'Remove kukuri:topic:second' }));

  await waitFor(() => {
    expect(
      screen.queryByRole('button', { name: 'kukuri:topic:second' })
    ).not.toBeInTheDocument();
    expectActiveTopic('kukuri:topic:demo');
  });
});

