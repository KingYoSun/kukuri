import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  getSocialConnectionsTabs,
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

test('post card shows friend of friend badge and author name fallback', async () => {
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'post-fof',
              envelope_id: 'envelope-fof',
              author_pubkey: 'a'.repeat(64),
              author_name: 'alice',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: true,
              object_kind: 'post',
              content: 'hello network',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-fof',
              audience_label: 'Public',
            },
          ],
        },
      })}
    />
  );

  expect(await screen.findByRole('button', { name: 'alice' })).toBeInTheDocument();
  expect(screen.getByText('friend of friend')).toBeInTheDocument();
});

test('profile social management updates follow and mute lists and muted authors disappear from content surfaces', async () => {
  const mutedAuthorPubkey = 'b'.repeat(64);
  const visibleAuthorPubkey = 'c'.repeat(64);
  const user = userEvent.setup();

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'post-muted-author',
              envelope_id: 'envelope-muted-author',
              author_pubkey: mutedAuthorPubkey,
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: true,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'mute this post',
              content_status: 'Available',
              attachments: [],
              created_at: 2,
              reply_to: null,
              root_id: 'post-muted-author',
              audience_label: 'Public',
            },
            {
              object_id: 'post-visible-author',
              envelope_id: 'envelope-visible-author',
              author_pubkey: visibleAuthorPubkey,
              author_name: 'carol',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'keep this post',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-visible-author',
              audience_label: 'Public',
            },
          ],
        },
        seedLiveSessions: {
          'kukuri:topic:demo': [
            {
              session_id: 'live-muted',
              host_pubkey: mutedAuthorPubkey,
              title: 'Muted Live',
              description: 'muted host session',
              status: 'Live',
              started_at: 2,
              ended_at: null,
              viewer_count: 0,
              joined_by_me: false,
              channel_id: null,
              audience_label: 'Public',
            },
            {
              session_id: 'live-visible',
              host_pubkey: visibleAuthorPubkey,
              title: 'Visible Live',
              description: 'visible host session',
              status: 'Live',
              started_at: 1,
              ended_at: null,
              viewer_count: 0,
              joined_by_me: false,
              channel_id: null,
              audience_label: 'Public',
            },
          ],
        },
        seedGameRooms: {
          'kukuri:topic:demo': [
            {
              room_id: 'room-muted',
              host_pubkey: mutedAuthorPubkey,
              title: 'Muted Room',
              description: 'muted host room',
              status: 'Waiting',
              phase_label: null,
              scores: [
                {
                  participant_id: 'participant-bob',
                  label: 'Bob',
                  score: 0,
                },
                {
                  participant_id: 'participant-carol',
                  label: 'Carol',
                  score: 0,
                },
              ],
              updated_at: 2,
              channel_id: null,
              audience_label: 'Public',
            },
            {
              room_id: 'room-visible',
              host_pubkey: visibleAuthorPubkey,
              title: 'Visible Room',
              description: 'visible host room',
              status: 'Waiting',
              phase_label: null,
              scores: [
                {
                  participant_id: 'participant-dave',
                  label: 'Dave',
                  score: 0,
                },
                {
                  participant_id: 'participant-erin',
                  label: 'Erin',
                  score: 0,
                },
              ],
              updated_at: 1,
              channel_id: null,
              audience_label: 'Public',
            },
          ],
        },
        authorSocialViews: {
          [mutedAuthorPubkey]: {
            name: 'bob',
            followed_by: true,
          },
          [visibleAuthorPubkey]: {
            name: 'carol',
          },
        },
      })}
    />
  );

  const mutedPostCard = (await screen.findByText('mute this post')).closest('article');
  if (!(mutedPostCard instanceof HTMLElement)) {
    throw new Error('muted author post card not found');
  }
  await user.click(within(mutedPostCard).getByRole('button', { name: 'Bookmark' }));
  await waitFor(() => {
    expect(within(mutedPostCard).getByRole('button', { name: 'Remove bookmark' })).toBeInTheDocument();
  });

  await selectWorkspace(user, 'Profile');
  await user.click(screen.getByRole('button', { name: '0 Following' }));

  const tabs = getSocialConnectionsTabs();
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Following' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  expect(screen.getByText('You are not following anyone yet.')).toBeInTheDocument();

  await user.click(within(tabs).getByRole('tab', { name: 'Followed' }));
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Followed' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  expect(
    screen.queryByText('Followed shows only followers already observed on this device.')
  ).not.toBeInTheDocument();

  let bobConnectionCard = screen.getByText(mutedAuthorPubkey).closest('article');
  if (!(bobConnectionCard instanceof HTMLElement)) {
    throw new Error('followed author card not found');
  }
  await user.click(within(bobConnectionCard).getByRole('button', { name: 'Follow' }));
  await waitFor(() => {
    const refreshedCard = screen.getByText(mutedAuthorPubkey).closest('article');
    expect(refreshedCard).toBeInstanceOf(HTMLElement);
    expect(
      within(refreshedCard as HTMLElement).getByRole('button', { name: 'Unfollow' })
    ).toBeInTheDocument();
  });

  bobConnectionCard = screen.getByText(mutedAuthorPubkey).closest('article');
  if (!(bobConnectionCard instanceof HTMLElement)) {
    throw new Error('refreshed followed author card not found');
  }
  await user.click(within(bobConnectionCard).getByRole('button', { name: 'Mute' }));
  await waitFor(() => {
    const refreshedCard = screen.getByText(mutedAuthorPubkey).closest('article');
    expect(refreshedCard).toBeInstanceOf(HTMLElement);
    expect(within(refreshedCard as HTMLElement).getByText('Muted')).toBeInTheDocument();
    expect(
      within(refreshedCard as HTMLElement).getByRole('button', { name: 'Unmute' })
    ).toBeInTheDocument();
  });

  await user.click(within(tabs).getByRole('tab', { name: 'Following' }));
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Following' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  bobConnectionCard = screen.getByText(mutedAuthorPubkey).closest('article');
  if (!(bobConnectionCard instanceof HTMLElement)) {
    throw new Error('following author card not found');
  }
  expect(within(bobConnectionCard).getByRole('button', { name: 'Unfollow' })).toBeInTheDocument();

  await user.click(within(tabs).getByRole('tab', { name: 'Muted' }));
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Muted' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  bobConnectionCard = screen.getByText(mutedAuthorPubkey).closest('article');
  if (!(bobConnectionCard instanceof HTMLElement)) {
    throw new Error('muted author card not found');
  }
  expect(within(bobConnectionCard).getByRole('button', { name: 'Unmute' })).toBeInTheDocument();
  expect(within(bobConnectionCard).getByText('Muted')).toBeInTheDocument();

  await selectWorkspace(user, 'Timeline');
  await waitFor(() => {
    expect(screen.queryByText('mute this post')).not.toBeInTheDocument();
  });
  expect(screen.getByText('keep this post')).toBeInTheDocument();

  await selectTimelineView(user, 'Bookmarks');
  await waitFor(() => {
    expect(screen.getByText('No bookmarked posts yet.')).toBeInTheDocument();
  });

  await selectWorkspace(user, 'Live');
  await waitFor(() => {
    expect(screen.queryByText('Muted Live')).not.toBeInTheDocument();
  });
  expect(screen.getByText('Visible Live')).toBeInTheDocument();

  await selectWorkspace(user, 'Game');
  await waitFor(() => {
    expect(screen.queryByText('Muted Room')).not.toBeInTheDocument();
  });
  expect(screen.queryByText('Visible Room')).not.toBeInTheDocument();
  expect(screen.getByText('Metaverse Rooms')).toBeInTheDocument();
});

test('author detail shows via authors and follow action updates relationship', async () => {
  const authorPubkey = 'b'.repeat(64);
  const viaA = 'c'.repeat(64);
  const viaB = 'd'.repeat(64);
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-author',
          envelope_id: 'envelope-author',
          author_pubkey: authorPubkey,
          author_name: 'bob',
          author_display_name: null,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: true,
          object_kind: 'post',
          content: 'author detail',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-author',
          audience_label: 'Public',
        },
      ],
    },
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        friend_of_friend: true,
        friend_of_friend_via_pubkeys: [viaA, viaB],
      },
    },
  });
  const user = userEvent.setup();

  render(<App api={api} />);

  await user.click(await screen.findByRole('button', { name: 'bob' }));

  expect(await screen.findByTestId('author-detail-avatar')).toBeInTheDocument();
  expect(screen.getByText(`${viaA.slice(0, 12)}, ${viaB.slice(0, 12)}`)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Follow' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Follow' }));

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Unfollow' })).toBeInTheDocument();
  });
  expect(screen.getAllByText('following').length).toBeGreaterThan(0);
});

test('author detail mute toggle updates the selected author state', async () => {
  const authorPubkey = 'b'.repeat(64);
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'post-author-mute',
              envelope_id: 'envelope-author-mute',
              author_pubkey: authorPubkey,
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'author mute target',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-author-mute',
              audience_label: 'Public',
            },
          ],
        },
        authorSocialViews: {
          [authorPubkey]: {
            name: 'bob',
            about: 'author detail stays visible while muted',
          },
        },
      })}
    />
  );

  await user.click(await screen.findByRole('button', { name: 'bob' }));

  const authorPane = await screen.findByRole('complementary', { name: 'Author' });
  expect(within(authorPane).getByRole('button', { name: 'Mute' })).toBeInTheDocument();

  await user.click(within(authorPane).getByRole('button', { name: 'Mute' }));

  await waitFor(() => {
    expect(within(authorPane).getByRole('button', { name: 'Unmute' })).toBeInTheDocument();
  });
  expect(within(authorPane).getByText('author detail stays visible while muted')).toBeInTheDocument();
});

