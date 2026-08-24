import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  buildPaginatedPost,
  createDeferred,
  openChannelManager,
  openControlCenter,
  paginatePosts,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { DesktopApi, PostView, TimelineCursor, TimelineView } from '@/lib/api';
import { REFRESH_INTERVAL_MS } from '@/shell/store';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

test('timeline polling does not overlap refreshes while a refresh is in flight', async () => {
  vi.useFakeTimers();
  const api = createDesktopMockApi();
  const listTimelineDeferreds: Array<ReturnType<typeof createDeferred<TimelineView>>> = [];
  const listTimelineSpy = vi.fn(() => {
    const deferred = createDeferred<TimelineView>();
    listTimelineDeferreds.push(deferred);
    return deferred.promise;
  });
  api.listTimeline = listTimelineSpy;

  const view = render(<App api={api} />);

  await vi.advanceTimersByTimeAsync(0);
  expect(listTimelineSpy).toHaveBeenCalledTimes(2);

  await vi.advanceTimersByTimeAsync(REFRESH_INTERVAL_MS * 3);
  expect(listTimelineSpy).toHaveBeenCalledTimes(2);

  const initialDeferreds = [...listTimelineDeferreds];
  for (const deferred of initialDeferreds) {
    deferred.resolve({
      items: [],
      next_cursor: null,
    });
  }

  await Promise.resolve();
  await vi.advanceTimersByTimeAsync(0);
  expect(listTimelineSpy).toHaveBeenCalledTimes(2);

  await vi.advanceTimersByTimeAsync(REFRESH_INTERVAL_MS);
  expect(listTimelineSpy).toHaveBeenCalledTimes(4);

  await vi.advanceTimersByTimeAsync(REFRESH_INTERVAL_MS * 2);
  expect(listTimelineSpy).toHaveBeenCalledTimes(4);

  view.unmount();
});

test('timeline buffers remote posts until the pending banner is applied', async () => {
  const user = userEvent.setup();
  const olderPost: PostView = {
    object_id: 'post-old',
    envelope_id: 'envelope-old',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'older post',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-old',
    channel_id: null,
    audience_label: 'Public',
  };
  const newerPost: PostView = {
    ...olderPost,
    object_id: 'post-new',
    envelope_id: 'envelope-new',
    content: 'newer post',
    created_at: 2,
    root_id: 'post-new',
  };
  let timelineItems = [olderPost];
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': timelineItems,
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async listTimeline(topic, cursor, limit, scope) {
      if (topic !== 'kukuri:topic:demo') {
        return baseApi.listTimeline(topic, cursor, limit, scope);
      }
      return {
        items: timelineItems.map((item) => ({ ...item, attachments: [...item.attachments] })),
        next_cursor: null,
      };
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('older post')).toBeInTheDocument();

  timelineItems = [newerPost, olderPost];
  window.dispatchEvent(new Event('focus'));

  expect(await screen.findByRole('button', { name: 'Show 1 new post' })).toBeInTheDocument();
  expect(screen.queryByText('newer post')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Show 1 new post' }));

  await waitFor(() => {
    expect(screen.getByText('newer post')).toBeInTheDocument();
  });
});

test('pending timeline snapshots apply all unseen posts from the latest first page', async () => {
  const user = userEvent.setup();
  const olderPost: PostView = {
    object_id: 'post-old',
    envelope_id: 'envelope-old',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'older post',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-old',
    channel_id: null,
    audience_label: 'Public',
  };
  const firstNewPost: PostView = {
    ...olderPost,
    object_id: 'post-new-a',
    envelope_id: 'envelope-new-a',
    content: 'first unseen post',
    created_at: 2,
    root_id: 'post-new-a',
  };
  const secondNewPost: PostView = {
    ...olderPost,
    object_id: 'post-new-b',
    envelope_id: 'envelope-new-b',
    content: 'second unseen post',
    created_at: 3,
    root_id: 'post-new-b',
  };
  let timelineItems = [olderPost];
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': timelineItems,
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async listTimeline(topic, cursor, limit, scope) {
      if (topic !== 'kukuri:topic:demo') {
        return baseApi.listTimeline(topic, cursor, limit, scope);
      }
      return {
        items: timelineItems.map((item) => ({ ...item, attachments: [...item.attachments] })),
        next_cursor: null,
      };
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('older post')).toBeInTheDocument();

  timelineItems = [secondNewPost, firstNewPost, olderPost];
  window.dispatchEvent(new Event('focus'));
  expect(await screen.findByRole('button', { name: 'Show 2 new post' })).toBeInTheDocument();
  expect(screen.queryByText('first unseen post')).not.toBeInTheDocument();
  expect(screen.queryByText('second unseen post')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Show 2 new post' }));

  await waitFor(() => {
    expect(screen.getByText('first unseen post')).toBeInTheDocument();
    expect(screen.getByText('second unseen post')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: /Show \d+ new post/ })
    ).not.toBeInTheDocument();
  });
});

test('applying a pending timeline does not re-count the same post when a stale refresh completes later', async () => {
  const user = userEvent.setup();
  const olderPost: PostView = {
    object_id: 'post-old',
    envelope_id: 'envelope-old',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'older post',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-old',
    channel_id: null,
    audience_label: 'Public',
  };
  const newerPost: PostView = {
    ...olderPost,
    object_id: 'post-new',
    envelope_id: 'envelope-new',
    content: 'newer post',
    created_at: 2,
    root_id: 'post-new',
  };
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [olderPost],
    },
  });
  const inFlightRefresh = createDeferred<TimelineView>();
  let refreshPhase: 'initial' | 'buffered' | 'stale-in-flight' = 'initial';
  let staleRefreshStarted = 0;
  const api: DesktopApi = {
    ...baseApi,
    async listTimeline(topic, cursor, limit, scope) {
      if (topic !== 'kukuri:topic:demo') {
        return baseApi.listTimeline(topic, cursor, limit, scope);
      }
      if (refreshPhase === 'stale-in-flight') {
        staleRefreshStarted += 1;
        return inFlightRefresh.promise;
      }
      const items = refreshPhase === 'buffered' ? [newerPost, olderPost] : [olderPost];
      return {
        items: items.map((item) => ({ ...item, attachments: [...item.attachments] })),
        next_cursor: null,
      };
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('older post')).toBeInTheDocument();

  refreshPhase = 'buffered';
  window.dispatchEvent(new Event('focus'));

  const pendingButton = await screen.findByRole('button', { name: 'Show 1 new post' });
  expect(screen.queryByText('newer post')).not.toBeInTheDocument();

  refreshPhase = 'stale-in-flight';
  window.dispatchEvent(new Event('focus'));

  await waitFor(() => {
    expect(staleRefreshStarted).toBeGreaterThan(0);
  });

  await user.click(pendingButton);

  await waitFor(() => {
    expect(screen.getByText('newer post')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Show 1 new post' })
    ).not.toBeInTheDocument();
  });

  inFlightRefresh.resolve({
    items: [newerPost, olderPost].map((item) => ({ ...item, attachments: [...item.attachments] })),
    next_cursor: null,
  });

  await waitFor(() => {
    expect(screen.getByText('newer post')).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Show 1 new post' })
    ).not.toBeInTheDocument();
  });
});

test('authoritative replacements for syncing posts do not increment the pending banner', async () => {
  const olderPost: PostView = {
    object_id: 'post-old',
    envelope_id: 'envelope-old',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'older post',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-old',
    channel_id: null,
    audience_label: 'Public',
  };
  const syncingPost: PostView = {
    ...olderPost,
    object_id: 'local-syncing-post',
    envelope_id: 'local-syncing-envelope',
    content: 'syncing placeholder',
    created_at: 3,
    root_id: 'local-syncing-post',
    local_id: 'local-syncing-post',
    local_state: 'syncing',
    server_object_id: 'server-post',
  };
  const authoritativeReplacement: PostView = {
    ...olderPost,
    object_id: 'server-post',
    envelope_id: 'server-envelope',
    content: 'authoritative replacement',
    created_at: 3,
    root_id: 'server-post',
  };
  let timelineItems = [syncingPost, olderPost];
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': timelineItems,
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async listTimeline(topic, cursor, limit, scope) {
      if (topic !== 'kukuri:topic:demo') {
        return baseApi.listTimeline(topic, cursor, limit, scope);
      }
      return {
        items: timelineItems.map((item) => ({ ...item, attachments: [...item.attachments] })),
        next_cursor: null,
      };
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('syncing placeholder')).toBeInTheDocument();
  expect(screen.getByText('older post')).toBeInTheDocument();

  timelineItems = [authoritativeReplacement, olderPost];
  window.dispatchEvent(new Event('focus'));

  await waitFor(() => {
    expect(screen.getByText('authoritative replacement')).toBeInTheDocument();
    expect(screen.queryByText('syncing placeholder')).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Show 1 new post' })
    ).not.toBeInTheDocument();
  });
});

test('private channel timeline keeps scope-separated posts and pending counts from public', async () => {
  const user = userEvent.setup();
  const publicPost: PostView = {
    object_id: 'post-public',
    envelope_id: 'envelope-public',
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    is_threadable: true,
    content: 'public post',
    content_status: 'Available',
    attachments: [],
    created_at: 1,
    reply_to: null,
    root_id: 'post-public',
    channel_id: null,
    audience_label: 'Public',
  };
  const channelPost: PostView = {
    ...publicPost,
    object_id: 'post-channel',
    envelope_id: 'envelope-channel',
    content: 'channel post',
    created_at: 2,
    root_id: 'post-channel',
    channel_id: 'channel-1',
    audience_label: 'core',
  };
  const channelNewPost: PostView = {
    ...channelPost,
    object_id: 'post-channel-new',
    envelope_id: 'envelope-channel-new',
    content: 'channel post new',
    created_at: 3,
    root_id: 'post-channel-new',
  };
  const publicTimelineItems = [publicPost];
  let channelTimelineItems = [channelPost];
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [publicPost, channelPost],
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async listTimeline(topic, cursor, limit, scope) {
      if (topic !== 'kukuri:topic:demo') {
        return baseApi.listTimeline(topic, cursor, limit, scope);
      }
      if (scope?.kind === 'channel') {
        return {
          items: channelTimelineItems.map((item) => ({ ...item, attachments: [...item.attachments] })),
          next_cursor: null,
        };
      }
      return {
        items: publicTimelineItems.map((item) => ({ ...item, attachments: [...item.attachments] })),
        next_cursor: null,
      };
    },
  };

  render(<App api={api} />);

  expect(await screen.findByText('public post')).toBeInTheDocument();
  expect(screen.queryByText('channel post')).not.toBeInTheDocument();

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
    ).not.toBeInTheDocument();
    expect(screen.getByText('channel post')).toBeInTheDocument();
  });
  expect(screen.getByText('public post')).toBeInTheDocument();

  channelTimelineItems = [channelNewPost, channelPost];
  window.dispatchEvent(new Event('focus'));

  expect(await screen.findByRole('button', { name: 'Show 1 new post' })).toBeInTheDocument();
  expect(screen.getByText('public post')).toBeInTheDocument();
  expect(screen.queryByText('channel post new')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Show 1 new post' }));

  await waitFor(() => {
    expect(screen.getByText('channel post new')).toBeInTheDocument();
  });
  expect(screen.getByText('public post')).toBeInTheDocument();

  const controlCenter = await openControlCenter(user);
  const topicItem = within(controlCenter).getByRole('button', { name: 'demo' }).closest('li');
  if (!(topicItem instanceof HTMLElement)) {
    throw new Error('active topic item not found');
  }
  const publicButton = within(topicItem).getByText('Public').closest('button');
  if (!(publicButton instanceof HTMLButtonElement)) {
    throw new Error('public scope button not found');
  }

  await user.click(publicButton);

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
    expect(screen.getByText('public post')).toBeInTheDocument();
  });
  expect(screen.getByText('channel post')).toBeInTheDocument();
  expect(screen.getByText('channel post new')).toBeInTheDocument();
});

test('background refresh preserves loaded timeline pages and does not restore a stale load-more cursor', async () => {
  const user = userEvent.setup();
  const paginatedPosts = Array.from({ length: 25 }, (_, index) =>
    buildPaginatedPost(25 - index, {
      object_id: `paginated-post-${25 - index}`,
      envelope_id: `paginated-envelope-${25 - index}`,
      root_id: `paginated-post-${25 - index}`,
      reply_to: null,
      object_kind: 'post',
    })
  );
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': paginatedPosts,
    },
  });
  api.listTimeline = vi.fn(
    async (topic: string, cursor: TimelineCursor | null, limit = 20) => {
      return paginatePosts(
        topic === 'kukuri:topic:demo' ? paginatedPosts : [],
        cursor,
        limit
      );
    }
  );

  render(<App api={api} />);

  await screen.findByText('paginated post 25');
  expect(screen.queryByText('paginated post 1')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Load more' }));

  await waitFor(() => {
    expect(screen.getByText('paginated post 1')).toBeInTheDocument();
  });
  expect(screen.queryByRole('button', { name: 'Load more' })).not.toBeInTheDocument();

  window.dispatchEvent(new Event('focus'));

  await waitFor(() => {
    expect(screen.getByText('paginated post 1')).toBeInTheDocument();
  });
  expect(screen.queryByRole('button', { name: 'Load more' })).not.toBeInTheDocument();
});


// Issue #765 T5: 表示中の背景(非 active)Timeline Column の scope も定期 refresh される。
test('periodic refresh also fetches visible background Timeline Column scopes', async () => {
  vi.useFakeTimers();
  const { WORKSPACE_LAYOUT_STORAGE_KEY } = await import('@/shell/workspacePersistence');
  const { columnIdentityId } = await import('@/shell/slices/workspace');
  const demoScope = { topicId: 'kukuri:topic:demo', channelId: null };
  const irohScope = { topicId: 'kukuri:topic:iroh', channelId: null };
  window.localStorage.setItem(
    WORKSPACE_LAYOUT_STORAGE_KEY,
    JSON.stringify({
      version: 1,
      activeColumnId: columnIdentityId('timeline', demoScope),
      columns: [
        {
          id: columnIdentityId('timeline', demoScope),
          kind: 'timeline',
          scope: demoScope,
          pinned: true,
          preferredDesktopSpan: 1,
        },
        {
          id: columnIdentityId('timeline', irohScope),
          kind: 'timeline',
          scope: irohScope,
          pinned: true,
          preferredDesktopSpan: 1,
        },
      ],
    })
  );
  const api = createDesktopMockApi();
  const listTimelineSpy = vi.spyOn(api, 'listTimeline');

  render(<App api={api} />);
  await vi.advanceTimersByTimeAsync(0);
  await vi.advanceTimersByTimeAsync(REFRESH_INTERVAL_MS + 50);

  const refreshedTopics = new Set(listTimelineSpy.mock.calls.map(([topic]) => topic));
  expect(refreshedTopics.has('kukuri:topic:demo')).toBe(true);
  // 背景の iroh Timeline Column も定期 refresh の対象になる。
  expect(refreshedTopics.has('kukuri:topic:iroh')).toBe(true);
});
