import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  buildPaginatedPost,
  getDetailPane,
  installObjectUrlMocks,
  paginatePosts,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { TimelineCursor } from '@/lib/api';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('clicking a timeline post opens thread and author detail flows in the context pane', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [
            {
              object_id: 'post-thread-open',
              envelope_id: 'envelope-thread-open',
              author_pubkey: 'b'.repeat(64),
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: true,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'open thread from timeline',
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
        authorSocialViews: {
          ['b'.repeat(64)]: {
            name: 'bob',
            display_name: null,
            about: 'author detail from timeline',
            following: false,
            followed_by: true,
            mutual: false,
            friend_of_friend: false,
            friend_of_friend_via_pubkeys: [],
          },
        },
      })}
    />
  );

  await user.click(await screen.findByText('open thread from timeline'));
  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
  });
  expect(within(getDetailPane('Thread')).getByText('open thread from timeline')).toBeInTheDocument();

  await user.click(within(getDetailPane('Thread')).getAllByRole('button', { name: 'bob' })[0]);

  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(within(getDetailPane('Author')).getByTestId('author-detail-avatar')).toBeInTheDocument();
  expect(within(getDetailPane('Author')).getByText('author detail from timeline')).toBeInTheDocument();
});

test('thread focus auto-scroll runs only once even when the thread loads additional pages', async () => {
  const user = userEvent.setup();
  const scrollIntoView = vi.fn();
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: scrollIntoView,
  });
  const threadPosts = Array.from({ length: 35 }, (_, index) =>
    buildPaginatedPost(35 - index)
  );
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:general': threadPosts,
    },
  });
  api.listThread = vi.fn(async (_topic: string, threadId: string, cursor: TimelineCursor | null, limit = 30) => {
    if (threadId !== 'paginated-post-1') {
      return { items: [], next_cursor: null };
    }
    return paginatePosts(threadPosts, cursor, limit);
  });

  renderAtHash(
    '#/timeline?topic=kukuri%3Atopic%3Ageneral&context=thread&threadId=paginated-post-1&focusObjectId=paginated-post-11',
    api
  );

  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
  });
  await waitFor(() => {
    expect(scrollIntoView).toHaveBeenCalledTimes(2);
  });
  const initialScrollCount = scrollIntoView.mock.calls.length;

  await user.click(within(getDetailPane('Thread')).getByRole('button', { name: 'Load more' }));

  await waitFor(() => {
    expect(within(getDetailPane('Thread')).getByText('paginated post 1')).toBeInTheDocument();
  });
  expect(scrollIntoView).toHaveBeenCalledTimes(initialScrollCount);
});

test('timeline author detail opens as one Column, and thread author detail opens to its right', async () => {
  const user = userEvent.setup();
  const authorPubkey = 'a'.repeat(64);
  const createApi = () =>
    createDesktopMockApi({
      seedPosts: {
        'kukuri:topic:general': [
          {
            object_id: 'context-post',
            envelope_id: 'envelope-context-post',
            author_pubkey: authorPubkey,
            author_name: 'alice',
            author_display_name: null,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            object_kind: 'post',
            content: 'context body',
            content_status: 'Available',
            attachments: [],
            created_at: 1,
            reply_to: null,
            root_id: 'context-post',
            audience_label: 'Public',
          },
        ],
      },
      authorSocialViews: {
        [authorPubkey]: {
          name: 'alice',
        },
      },
    });

  const { unmount } = renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', createApi());

  await user.click(await screen.findByRole('button', { name: 'alice' }));
  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(screen.queryByRole('region', { name: /^Thread Column,/ })).not.toBeInTheDocument();

  unmount();
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ageneral', createApi());

  await user.click(await screen.findByRole('button', { name: /context body/i }));
  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
  });
  expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();

  await user.click(within(getDetailPane('Thread')).getByRole('button', { name: 'alice' }));
  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(getDetailPane('Thread')).toBeInTheDocument();
});

test('author avatar blob stays visible on the timeline after the author pane closes', async () => {
  installObjectUrlMocks();

  const authorPubkey = 'b'.repeat(64);
  const user = userEvent.setup();

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [
            {
              object_id: 'post-author-avatar',
              envelope_id: 'envelope-author-avatar',
              author_pubkey: authorPubkey,
              author_name: 'bob',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'avatar persistence',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-author-avatar',
              audience_label: 'Public',
            },
          ],
        },
        authorSocialViews: {
          [authorPubkey]: {
            name: 'bob',
            picture_asset: {
              hash: 'avatar-hash',
              mime: 'image/png',
              bytes: 64,
              role: 'profile_avatar',
            },
          },
        },
      })}
    />
  );

  await user.click(await screen.findByRole('button', { name: 'bob' }));

  const timelineAvatars = await screen.findAllByTestId('post-author-avatar-author-avatar');
  await waitFor(() => {
    expect(
      timelineAvatars.some((avatar) => avatar.querySelector('img')?.getAttribute('src') === 'blob:mock-1')
    ).toBe(true);
  });

  const authorPane = getDetailPane('Author');
  await user.click(within(authorPane).getByRole('button', { name: 'Close Profile' }));

  await waitFor(() => {
    expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();
    expect(
      screen
        .getAllByTestId('post-author-avatar-author-avatar')
        .some((avatar) => avatar.querySelector('img')?.getAttribute('src') === 'blob:mock-1')
    ).toBe(true);
  });
});

test('remote author avatar appears on the timeline without opening the author pane', async () => {
  installObjectUrlMocks();

  const authorPubkey = 'c'.repeat(64);

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:general': [
            {
              object_id: 'post-inline-avatar',
              envelope_id: 'envelope-inline-avatar',
              author_pubkey: authorPubkey,
              author_name: 'carol',
              author_display_name: null,
              author_picture_asset: {
                hash: 'inline-avatar-hash',
                mime: 'image/png',
                bytes: 64,
                role: 'profile_avatar',
              },
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'inline avatar hydration',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'post-inline-avatar',
              audience_label: 'Public',
            },
          ],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(
      screen
        .getByTestId('post-inline-avatar-author-avatar')
        .querySelector('img')
        ?.getAttribute('src')
    ).toMatch(/^blob:mock-\d+$/);
  });
});

