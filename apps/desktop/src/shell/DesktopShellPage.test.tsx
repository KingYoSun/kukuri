import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';
import { buildChannelAccessPreviewDeepLink } from '@/lib/internalLinks';
import { App } from '@/App';
import type {
  AttachmentView,
  BlobViewStatus,
  CreateAttachmentInput,
  DesktopApi,
  DirectMessageMessageView,
  JoinedPrivateChannelView,
  NotificationView,
  PostView,
  TimelineCursor,
  TimelineView,
} from '@/lib/api';
import { REFRESH_INTERVAL_MS } from '@/shell/store';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

function setViewportWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: width,
  });
  window.dispatchEvent(new Event('resize'));
}

function createDeferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  let reject!: (reason?: unknown) => void;
  const promise = new Promise<T>((nextResolve, nextReject) => {
    resolve = nextResolve;
    reject = nextReject;
  });
  return {
    promise,
    reject,
    resolve,
  };
}

function buildImagePost(overrides?: Partial<PostView>): PostView {
  const attachment: AttachmentView = {
    hash: 'a'.repeat(64),
    mime: 'image/png',
    bytes: 2048,
    role: 'image_original',
    status: 'Missing',
  };

  return {
    object_id: 'image-post',
    envelope_id: 'envelope-image-post',
    author_pubkey: 'f'.repeat(64),
    author_name: null,
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    content: '[blob pending]',
    content_status: 'Missing',
    attachments: [attachment],
    created_at: 1,
    reply_to: null,
    root_id: 'image-post',
    channel_id: null,
    audience_label: 'Public',
    ...overrides,
  };
}

function buildVideoPost(overrides?: Partial<PostView>): PostView {
  return {
    object_id: 'video-post',
    envelope_id: 'envelope-video-post',
    author_pubkey: 'e'.repeat(64),
    author_name: null,
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: 'post',
    content: 'video caption',
    content_status: 'Available',
    attachments: [
      {
        hash: 'v'.repeat(64),
        mime: 'video/mp4',
        bytes: 8192,
        role: 'video_manifest',
        status: 'Available',
      },
      {
        hash: 'p'.repeat(64),
        mime: 'image/jpeg',
        bytes: 1024,
        role: 'video_poster',
        status: 'Missing',
      },
    ],
    created_at: 2,
    reply_to: null,
    root_id: 'video-post',
    channel_id: null,
    audience_label: 'Public',
    ...overrides,
  };
}

function buildNotification(overrides?: Partial<NotificationView>): NotificationView {
  return {
    notification_id: 'notification-1',
    kind: 'reply',
    actor_pubkey: 'c'.repeat(64),
    actor_name: 'carol',
    actor_display_name: null,
    actor_picture: null,
    actor_picture_asset: null,
    source_envelope_id: 'notification-envelope-1',
    source_replica_id: 'replica:notification',
    topic_id: 'kukuri:topic:demo',
    channel_id: null,
    object_id: 'reply-1',
    thread_root_object_id: 'post-thread-open',
    dm_id: null,
    message_id: null,
    preview_text: 'notification preview',
    created_at: 1,
    received_at: 1,
    read_at: null,
    ...overrides,
  };
}

function buildPaginatedPost(index: number, overrides?: Partial<PostView>): PostView {
  return {
    object_id: `paginated-post-${index}`,
    envelope_id: `paginated-envelope-${index}`,
    author_pubkey: 'a'.repeat(64),
    author_name: 'alice',
    author_display_name: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    object_kind: index === 1 ? 'post' : 'comment',
    content: `paginated post ${index}`,
    content_status: 'Available',
    attachments: [],
    created_at: index,
    reply_to: index === 1 ? null : 'paginated-post-1',
    root_id: 'paginated-post-1',
    channel_id: null,
    audience_label: 'Public',
    ...overrides,
  };
}

function paginatePosts(posts: PostView[], cursor: TimelineCursor | null, limit: number): TimelineView {
  const startIndex = cursor
    ? posts.findIndex(
        (post) =>
          post.object_id === cursor.object_id && post.created_at === cursor.created_at
      ) + 1
    : 0;
  const normalizedStartIndex = startIndex > 0 ? startIndex : 0;
  const items = posts.slice(normalizedStartIndex, normalizedStartIndex + limit);
  return {
    items,
    next_cursor:
      normalizedStartIndex + limit < posts.length
        ? {
            created_at: items[items.length - 1]!.created_at,
            object_id: items[items.length - 1]!.object_id,
          }
        : null,
  };
}

function installObjectUrlMocks() {
  let sequence = 0;
  const createObjectUrl = vi
    .spyOn(URL, 'createObjectURL')
    .mockImplementation(() => `blob:mock-${++sequence}`);
  const revokeObjectUrl = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

  return { createObjectUrl, revokeObjectUrl };
}

function installSuccessfulPosterGenerationMocks() {
  Object.defineProperty(HTMLVideoElement.prototype, 'videoWidth', {
    configurable: true,
    get: () => 640,
  });
  Object.defineProperty(HTMLVideoElement.prototype, 'videoHeight', {
    configurable: true,
    get: () => 360,
  });
  Object.defineProperty(HTMLMediaElement.prototype, 'readyState', {
    configurable: true,
    get: () => 2,
  });
  vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(function load(
    this: HTMLMediaElement
  ) {
    queueMicrotask(() => {
      this.dispatchEvent(new Event('loadeddata'));
    });
  });
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    drawImage: vi.fn(),
  } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation((callback) => {
    callback(new Blob([Uint8Array.from([9, 8, 7, 6])], { type: 'image/jpeg' }));
  });
}

function installMetadataSeekPosterGenerationMocks() {
  Object.defineProperty(HTMLVideoElement.prototype, 'videoWidth', {
    configurable: true,
    get: () => 640,
  });
  Object.defineProperty(HTMLVideoElement.prototype, 'videoHeight', {
    configurable: true,
    get: () => 360,
  });
  Object.defineProperty(HTMLMediaElement.prototype, 'duration', {
    configurable: true,
    get: () => 12,
  });
  Object.defineProperty(HTMLMediaElement.prototype, 'readyState', {
    configurable: true,
    get: () => 2,
  });
  let currentTime = 0;
  Object.defineProperty(HTMLMediaElement.prototype, 'currentTime', {
    configurable: true,
    get: () => currentTime,
    set(this: HTMLMediaElement, value: number) {
      currentTime = value;
      queueMicrotask(() => {
        this.dispatchEvent(new Event('seeked'));
      });
    },
  });
  vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(function load(
    this: HTMLMediaElement
  ) {
    queueMicrotask(() => {
      this.dispatchEvent(new Event('loadedmetadata'));
    });
  });
  vi.spyOn(HTMLMediaElement.prototype, 'play').mockImplementation(async () => undefined);
  vi.spyOn(HTMLMediaElement.prototype, 'pause').mockImplementation(() => {});
  vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({
    drawImage: vi.fn(),
  } as unknown as CanvasRenderingContext2D);
  vi.spyOn(HTMLCanvasElement.prototype, 'toBlob').mockImplementation((callback) => {
    callback(new Blob([Uint8Array.from([9, 8, 7, 6])], { type: 'image/jpeg' }));
  });
}

function installFailedPosterGenerationMocks() {
  vi.spyOn(HTMLMediaElement.prototype, 'load').mockImplementation(function load(
    this: HTMLMediaElement
  ) {
    queueMicrotask(() => {
      this.dispatchEvent(new Event('error'));
    });
  });
}

async function openSettingsDrawer(user: ReturnType<typeof userEvent.setup>) {
  await waitFor(() => {
    expect(window.location.hash).toMatch(/^#\/(?:timeline|channels|live|game|profile)/);
  });
  await user.click(screen.getByTestId('shell-settings-trigger'));
  return await screen.findByRole('dialog', { name: 'Settings' });
}

async function openSettingsSection(
  user: ReturnType<typeof userEvent.setup>,
  section: 'appearance' | 'connectivity' | 'discovery' | 'community-node' | 'reactions'
) {
  const drawer = await openSettingsDrawer(user);
  await user.click(within(drawer).getByTestId(`settings-section-${section}`));
  return drawer;
}

function closestSection(element: HTMLElement) {
  const section = element.closest('section');
  if (!(section instanceof HTMLElement)) {
    throw new Error('expected enclosing section');
  }
  return section;
}

function renderAtHash(hash: string, api = createDesktopMockApi()) {
  window.history.replaceState(null, '', `/${hash}`);
  return render(<App api={api} />);
}

function expectActiveTopic(topic: string) {
  expect(window.location.hash).toContain(`topic=${encodeURIComponent(topic)}`);
  expect(screen.getByRole('button', { name: topic }).closest('li')).toHaveClass('topic-item-active');
}

function getWorkspaceTabs() {
  return screen.getByRole('tablist', { name: 'Workspaces' });
}

function getTimelineViewTabs() {
  return screen.getByRole('tablist', { name: 'Timeline views' });
}

function getSocialConnectionsTabs() {
  return screen.getByRole('tablist', { name: 'Social connections' });
}

async function selectWorkspace(
  user: ReturnType<typeof userEvent.setup>,
  label: 'Timeline' | 'Live' | 'Game' | 'Messages' | 'Profile'
) {
  await user.click(within(getWorkspaceTabs()).getByRole('tab', { name: label }));
  await waitFor(() => {
    expect(within(getWorkspaceTabs()).getByRole('tab', { name: label })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
}

async function selectTimelineView(
  user: ReturnType<typeof userEvent.setup>,
  label: 'Feed' | 'Bookmarks'
) {
  await user.click(within(getTimelineViewTabs()).getByRole('tab', { name: label }));
  await waitFor(() => {
    expect(within(getTimelineViewTabs()).getByRole('tab', { name: label })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
}

function getPrimaryNavigation() {
  return screen.getByLabelText('Primary navigation');
}

function getFloatingActionButton() {
  return screen.getByTestId('shell-fab');
}

async function openPublishDialog(user: ReturnType<typeof userEvent.setup>) {
  await user.click(getFloatingActionButton());
  return await screen.findByRole('dialog', { name: 'Publish' });
}

async function publishPost(user: ReturnType<typeof userEvent.setup>, content: string) {
  const dialog = await openPublishDialog(user);
  await user.type(within(dialog).getByPlaceholderText('Write a post'), content);
  await user.click(within(dialog).getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Publish' })).not.toBeInTheDocument();
  });
}

async function openChannelManager(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: 'Private Channels' }));
  return await screen.findByRole('dialog', { name: 'Create / Join Private Channel' });
}

function getChannelShareButton(
  dialog: HTMLElement,
  channelLabel?: string,
  audienceLabel?: string
) {
  void channelLabel;
  void audienceLabel;
  return within(dialog).getByRole('button', {
    name: 'Create share link',
  });
}

async function openChannelSettings(user: ReturnType<typeof userEvent.setup>, channelLabel: string) {
  await user.click(
    screen.getByRole('button', { name: `Open ${channelLabel} channel settings` })
  );
  return await screen.findByRole('dialog', { name: 'Channel Settings' });
}

async function openLiveCreateDialog(user: ReturnType<typeof userEvent.setup>) {
  await selectWorkspace(user, 'Live');
  await user.click(getFloatingActionButton());
  return await screen.findByRole('dialog', { name: 'Start Live' });
}

async function openGameCreateDialog(user: ReturnType<typeof userEvent.setup>) {
  await selectWorkspace(user, 'Game');
  await user.click(getFloatingActionButton());
  return await screen.findByRole('dialog', { name: 'Create Room' });
}

async function openNotificationsInbox(user: ReturnType<typeof userEvent.setup>) {
  await user.click(screen.getByRole('button', { name: /Notifications|通知/ }));
  return await screen.findByRole('heading', { name: 'Notifications' });
}

function getDetailPane(name: 'Thread' | 'Author') {
  return screen.getByRole('complementary', { name });
}

test('desktop shell can publish and render a post', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'hello desktop');

  await waitFor(() => {
    expect(screen.getByText('hello desktop')).toBeInTheDocument();
  });
  expectActiveTopic('kukuri:topic:demo');
  expect(screen.queryByTestId('shell-nav-trigger')).not.toBeInTheDocument();
  const demoTopic = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
  expect(demoTopic).not.toBeNull();
  expect(demoTopic).toHaveTextContent('joined / peers: 1');

  const drawer = await openSettingsSection(user, 'connectivity');
  expect(within(drawer).getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  const syncSection = closestSection(within(drawer).getByRole('heading', { name: 'Sync Status' }));
  expect(within(syncSection).getAllByText('Configured Peers').length).toBeGreaterThan(0);
  expect(within(syncSection).getByText('Connected to all configured peers')).toBeInTheDocument();
  expect(within(syncSection).getAllByText('peer-a').length).toBeGreaterThan(0);
});

test.each([
  {
    path: '#/timeline',
    workspaceLabel: 'Timeline',
    expectedControl: () => screen.getByRole('button', { name: 'Publish' }),
  },
  {
    path: '#/channels',
    workspaceLabel: 'Timeline',
    expectedControl: () => screen.getByRole('button', { name: 'Publish' }),
  },
  {
    path: '#/live',
    workspaceLabel: 'Live',
    expectedControl: () => screen.getByRole('button', { name: 'Start Live' }),
  },
  {
    path: '#/game',
    workspaceLabel: 'Game',
    expectedControl: () => screen.getByRole('button', { name: 'Create Room' }),
  },
  {
    path: '#/messages',
    workspaceLabel: 'Messages',
    expectedControl: () => screen.getByText('No direct messages yet.'),
  },
  {
    path: '#/profile',
    workspaceLabel: 'Profile',
    expectedControl: () => screen.getByRole('button', { name: 'Edit Profile' }),
  },
])(
  'primary hash route $path selects the correct section',
  async ({ path, workspaceLabel, expectedControl }) => {
    renderAtHash(path);

    const tab = within(getWorkspaceTabs()).getByRole('tab', { name: workspaceLabel });
    expect(expectedControl()).toBeInTheDocument();

    await waitFor(() => {
      expect(tab).toHaveAttribute('aria-selected', 'true');
      expect(window.location.hash).toBe(
        path === '#/channels'
          ? '#/timeline?topic=kukuri%3Atopic%3Ademo'
          : `${path}?topic=kukuri%3Atopic%3Ademo`
      );
    });
  }
);

test('sidebar notifications button shows unread count and opening inbox auto-marks read', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    notifications: [
      buildNotification({
        notification_id: 'notification-unread-1',
        preview_text: 'first unread notification',
      }),
      buildNotification({
        notification_id: 'notification-unread-2',
        kind: 'mention',
        object_id: 'mention-1',
        thread_root_object_id: 'mention-1',
        preview_text: 'second unread notification',
        received_at: 2,
      }),
    ],
  });

  render(<App api={api} />);

  const sidebarButton = screen.getByRole('button', { name: /Notifications/ });
  await waitFor(() => {
    expect(sidebarButton).toHaveTextContent('2');
  });

  await openNotificationsInbox(user);

  await waitFor(() => {
    expect(window.location.hash).toBe('#/notifications?topic=kukuri%3Atopic%3Ademo');
    expect(sidebarButton).toHaveTextContent('0');
  });
  expect(screen.getByText('first unread notification')).toBeInTheDocument();
  expect(screen.getByText('second unread notification')).toBeInTheDocument();
});

test('clicking the active notifications button returns to the previous route', async () => {
  const user = userEvent.setup();

  renderAtHash('#/profile?topic=kukuri%3Atopic%3Ademo');

  expect(await screen.findByRole('button', { name: 'Edit Profile' })).toBeInTheDocument();

  const notificationsButton = screen.getByRole('button', { name: /Notifications/ });
  await user.click(notificationsButton);

  await waitFor(() => {
    expect(window.location.hash).toBe('#/notifications?topic=kukuri%3Atopic%3Ademo');
  });
  expect(screen.getByRole('heading', { name: 'Notifications' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: /Notifications/ }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/profile?topic=kukuri%3Atopic%3Ademo');
  });
  expect(screen.getByRole('button', { name: 'Edit Profile' })).toBeInTheDocument();
});

test('notifications route renders inbox and marks unread notifications as read on load', async () => {
  const api = createDesktopMockApi({
    notifications: [
      buildNotification({
        notification_id: 'notification-read-on-load',
        preview_text: 'open from route',
      }),
    ],
  });
  const markAllNotificationsRead = vi.fn(api.markAllNotificationsRead);
  api.markAllNotificationsRead = markAllNotificationsRead;

  renderAtHash('#/notifications?topic=kukuri%3Atopic%3Ademo', api);

  expect(await screen.findByRole('heading', { name: 'Notifications' })).toBeInTheDocument();
  await waitFor(() => {
    expect(markAllNotificationsRead).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('button', { name: /Notifications/ })).toHaveTextContent('0');
  });
  expect(screen.getByText('open from route')).toBeInTheDocument();
});

test('notifications route renders an empty state when the inbox has no items', async () => {
  renderAtHash('#/notifications?topic=kukuri%3Atopic%3Ademo', createDesktopMockApi());

  expect(await screen.findByRole('heading', { name: 'Notifications' })).toBeInTheDocument();
  expect(await screen.findByText('No notifications yet.')).toBeInTheDocument();
});

test('notifications route surfaces a load error when the inbox request fails', async () => {
  const api = createDesktopMockApi();
  api.listNotifications = vi.fn().mockRejectedValue(new Error('load notifications exploded'));

  renderAtHash('#/notifications?topic=kukuri%3Atopic%3Ademo', api);

  expect(await screen.findByRole('heading', { name: 'Notifications' })).toBeInTheDocument();
  expect(await screen.findByText('load notifications exploded')).toBeInTheDocument();
});

test('notifications route surfaces auto-read errors and keeps unread state visible', async () => {
  const api = createDesktopMockApi({
    notifications: [
      buildNotification({
        notification_id: 'notification-auto-read-failure',
        preview_text: 'still unread notification',
      }),
    ],
  });
  api.markAllNotificationsRead = vi.fn().mockRejectedValue(new Error('mark read failed'));

  renderAtHash('#/notifications?topic=kukuri%3Atopic%3Ademo', api);

  expect(await screen.findByRole('heading', { name: 'Notifications' })).toBeInTheDocument();
  expect(await screen.findByText('mark read failed')).toBeInTheDocument();
  expect(screen.getByText('still unread notification')).toBeInTheDocument();
  expect(screen.getByText('Unread')).toBeInTheDocument();
  await waitFor(() => {
    expect(screen.getByRole('button', { name: /Notifications/ })).toHaveTextContent('1');
  });
});

test('reply notification click-through opens the source thread in timeline', async () => {
  const user = userEvent.setup();
  renderAtHash(
    '#/notifications?topic=kukuri%3Atopic%3Ademo',
    createDesktopMockApi({
      notifications: [
        buildNotification({
          notification_id: 'notification-thread-open',
          preview_text: 'open thread from notification',
          object_id: 'reply-1',
          thread_root_object_id: 'post-thread-open',
        }),
      ],
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
            content: 'thread root post',
            content_status: 'Available',
            attachments: [],
            created_at: 1,
            reply_to: null,
            root_id: 'post-thread-open',
            channel_id: null,
            audience_label: 'Public',
          },
          {
            object_id: 'reply-1',
            envelope_id: 'envelope-reply-1',
            author_pubkey: 'c'.repeat(64),
            author_name: 'carol',
            author_display_name: null,
            following: false,
            followed_by: false,
            mutual: false,
            friend_of_friend: false,
            object_kind: 'post',
            content: 'thread reply post',
            content_status: 'Available',
            attachments: [],
            created_at: 2,
            reply_to: 'post-thread-open',
            root_id: 'post-thread-open',
            channel_id: null,
            audience_label: 'Public',
          },
        ],
      },
    })
  );

  await screen.findByRole('heading', { name: 'Notifications' });
  await user.click(screen.getByText('open thread from notification'));

  await waitFor(() => {
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-thread-open'
    );
  });
  expect(getDetailPane('Thread')).toBeInTheDocument();
});

test('direct message notification click-through opens the messages pane', async () => {
  const user = userEvent.setup();
  const actorPubkey = 'd'.repeat(64);
  renderAtHash(
    '#/notifications?topic=kukuri%3Atopic%3Ademo',
    createDesktopMockApi({
      notifications: [
        buildNotification({
          notification_id: 'notification-dm-open',
          kind: 'direct_message',
          actor_pubkey: actorPubkey,
          actor_name: 'dan',
          topic_id: null,
          object_id: null,
          thread_root_object_id: null,
          dm_id: 'dm-1',
          message_id: 'message-1',
          preview_text: 'hello from dm notification',
        }),
      ],
      authorSocialViews: {
        [actorPubkey]: {
          name: 'dan',
          mutual: true,
          following: true,
          followed_by: true,
        },
      },
    })
  );

  await screen.findByRole('heading', { name: 'Notifications' });
  await user.click(screen.getByText('hello from dm notification'));

  await waitFor(() => {
    expect(window.location.hash).toContain('#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=');
  });
  expect(screen.getByPlaceholderText('Write a message')).toBeInTheDocument();
});

test('follow notification click-through opens the author pane from timeline', async () => {
  const user = userEvent.setup();
  const actorPubkey = 'e'.repeat(64);
  renderAtHash(
    '#/notifications?topic=kukuri%3Atopic%3Ademo',
    createDesktopMockApi({
      notifications: [
        buildNotification({
          notification_id: 'notification-follow-open',
          kind: 'followed',
          actor_pubkey: actorPubkey,
          actor_name: 'erin',
          topic_id: null,
          object_id: null,
          thread_root_object_id: null,
          preview_text: null,
        }),
      ],
      authorSocialViews: {
        [actorPubkey]: {
          name: 'erin',
          about: 'opened from follow notification',
        },
      },
    })
  );

  await screen.findByRole('heading', { name: 'Notifications' });
  await user.click(screen.getByText('Started following you.'));

  await waitFor(() => {
    expect(window.location.hash).toBe(
      `#/timeline?topic=kukuri%3Atopic%3Ademo&context=author&authorPubkey=${actorPubkey}`
    );
  });
  expect(getDetailPane('Author')).toBeInTheDocument();
  expect(screen.getByText('opened from follow notification')).toBeInTheDocument();
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

test('invalid hash routes fall back to the active public timeline and normalize the URL', async () => {
  renderAtHash(
    '#/unknown?topic=missing-topic&timelineScope=channel:missing&composeTarget=channel:missing&context=author&authorPubkey=bad&settings=invalid'
  );

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
  expectActiveTopic('kukuri:topic:demo');
  expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
});

test('invalid timelineView normalizes to the feed route', async () => {
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo&timelineView=invalid');

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
  expect(within(getTimelineViewTabs()).getByRole('tab', { name: 'Feed' })).toHaveAttribute(
    'aria-selected',
    'true'
  );
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

test('thread context restores from the hash route and loads the requested thread for the active topic', async () => {
  renderAtHash(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-thread-open',
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
    })
  );

  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
  });
  expect(within(getDetailPane('Thread')).getAllByText('open thread from timeline').length).toBeGreaterThan(0);
});

test('author context restores from the hash route when a valid author pubkey is supplied', async () => {
  const authorPubkey = 'b'.repeat(64);
  renderAtHash(
    `#/timeline?topic=kukuri%3Atopic%3Ademo&context=author&authorPubkey=${authorPubkey}`,
    createDesktopMockApi({
      authorSocialViews: {
        [authorPubkey]: {
          name: 'bob',
          display_name: null,
          about: 'author detail from route restore',
          following: false,
          followed_by: true,
          mutual: false,
          friend_of_friend: false,
          friend_of_friend_via_pubkeys: [],
        },
      },
    })
  );

  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(within(getDetailPane('Author')).getByText('author detail from route restore')).toBeInTheDocument();
});

test('profile edit route restores the editor and keeps overview as the default profile mode', async () => {
  renderAtHash('#/profile?topic=kukuri%3Atopic%3Ademo&profileMode=edit');

  expect(screen.getByPlaceholderText('Visible label')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Back to profile' })).toBeInTheDocument();
});

test('profile connections route restores the requested view', async () => {
  const authorPubkey = 'b'.repeat(64);
  renderAtHash(
    '#/profile?topic=kukuri%3Atopic%3Ademo&profileMode=connections&connectionsView=muted',
    createDesktopMockApi({
      authorSocialViews: {
        [authorPubkey]: {
          name: 'bob',
          muted: true,
        },
      },
    })
  );

  const tabs = await screen.findByRole('tablist', { name: 'Social connections' });
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Muted' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(window.location.hash).toBe(
      '#/profile?topic=kukuri%3Atopic%3Ademo&profileMode=connections&connectionsView=muted'
    );
  });
  expect(
    screen.queryByText('Muted is local-only and is not shared with other devices.')
  ).not.toBeInTheDocument();
  expect(screen.getByText(authorPubkey)).toBeInTheDocument();
});

test('invalid profile connections view normalizes to following', async () => {
  const authorPubkey = 'b'.repeat(64);
  renderAtHash(
    '#/profile?topic=kukuri%3Atopic%3Ademo&profileMode=connections&connectionsView=invalid',
    createDesktopMockApi({
      authorSocialViews: {
        [authorPubkey]: {
          name: 'bob',
          following: true,
        },
      },
    })
  );

  const tabs = await screen.findByRole('tablist', { name: 'Social connections' });
  await waitFor(() => {
    expect(within(tabs).getByRole('tab', { name: 'Following' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(window.location.hash).toBe(
      '#/profile?topic=kukuri%3Atopic%3Ademo&profileMode=connections&connectionsView=following'
    );
  });
  expect(screen.getByText(authorPubkey)).toBeInTheDocument();
});

test('invalid nested author route keeps the thread pane and normalizes only the author param', async () => {
  renderAtHash(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-thread-open&authorPubkey=bad',
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
    })
  );

  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=post-thread-open'
    );
  });
  expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();
});

test('invalid thread route closes the entire detail stack and normalizes the URL', async () => {
  renderAtHash(
    `#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=missing-thread&authorPubkey=${'b'.repeat(64)}`,
    createDesktopMockApi({
      authorSocialViews: {
        ['b'.repeat(64)]: {
          name: 'bob',
        },
      },
    })
  );

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
  expect(screen.queryByRole('complementary', { name: 'Thread' })).not.toBeInTheDocument();
  expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();
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

test('desktop shell can enter reply mode and render reply state', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'root post');
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyDialog = await screen.findByRole('dialog', { name: 'Reply' });
  expect(within(replyDialog).getByPlaceholderText('Write a reply')).toBeInTheDocument();
  expect(within(replyDialog).getByText('Replying')).toBeInTheDocument();
  expect(within(replyDialog).getByText('Original post')).toBeInTheDocument();
  expect(within(replyDialog).getByText('root post')).toBeInTheDocument();

  const replyInput = within(replyDialog).getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  await user.click(within(composer).getByRole('button', { name: 'Reply' }));

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(screen.getAllByRole('button', { name: 'Reply' }).length).toBeGreaterThan(0);
});

test('compose dialog stays width-safe when the source post contains a long token', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const longContent = 'channel_payload_'.repeat(48);

  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  fireEvent.change(within(publishDialog).getByPlaceholderText('Write a post'), {
    target: { value: longContent },
  });
  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Publish' })).not.toBeInTheDocument();
  });
  await waitFor(() => {
    expect(screen.getByText(longContent)).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyDialog = await screen.findByRole('dialog', { name: 'Reply' });

  expect(replyDialog).toHaveClass('shell-compose-dialog');
  expect(within(replyDialog).getAllByText(longContent)[0]).toHaveClass('post-copy-wrap');
});

test('reply publish reloads thread only once after a successful submit', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalListThread = api.listThread;
  const listThreadSpy = vi.fn((topic, threadId, cursor, limit) =>
    originalListThread(topic, threadId, cursor, limit)
  );
  api.listThread = listThreadSpy;

  render(<App api={api} />);

  await publishPost(user, 'root post');
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getAllByRole('button', { name: 'Reply' })[0]);
  const replyDialog = await screen.findByRole('dialog', { name: 'Reply' });
  const threadCallsBeforeSubmit = listThreadSpy.mock.calls.length;

  const replyInput = within(replyDialog).getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  await user.click(within(composer).getByRole('button', { name: 'Reply' }));

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(listThreadSpy.mock.calls.length - threadCallsBeforeSubmit).toBe(1);
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

test('publish dialog closes without waiting for a timeline refresh after submit', async () => {
  const api = createDesktopMockApi();
  const listTimelineDeferreds: Array<ReturnType<typeof createDeferred<TimelineView>>> = [];
  api.listTimeline = vi.fn(() => {
    const deferred = createDeferred<TimelineView>();
    listTimelineDeferreds.push(deferred);
    return deferred.promise;
  });

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.type(within(publishDialog).getByPlaceholderText('Write a post'), 'publish without wait');
  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Publish' })).not.toBeInTheDocument();
  });
});

test('publish refreshes the active timeline without reloading full shell data', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalListDirectMessages = api.listDirectMessages;
  const listDirectMessagesSpy = vi.fn(() => originalListDirectMessages());
  api.listDirectMessages = listDirectMessagesSpy;

  render(<App api={api} />);

  await waitFor(() => {
    expect(listDirectMessagesSpy).toHaveBeenCalledTimes(0);
  });

  const publishDialog = await openPublishDialog(user);
  await user.type(within(publishDialog).getByPlaceholderText('Write a post'), 'local refresh post');
  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(screen.getByText('local refresh post')).toBeInTheDocument();
  });
  expect(listDirectMessagesSpy).toHaveBeenCalledTimes(0);
});

test('desktop shell can create a simple repost from timeline', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalCreateRepost = api.createRepost;
  const createRepostSpy = vi.fn((topic, sourceTopic, sourceObjectId, commentary) =>
    originalCreateRepost(topic, sourceTopic, sourceObjectId, commentary)
  );
  api.createRepost = createRepostSpy;

  render(<App api={api} />);

  await publishPost(user, 'source post');
  const sourcePost = await screen.findByText('source post');
  const card = sourcePost.closest('article');
  if (!card) {
    throw new Error('source post card not found');
  }

  await user.click(within(card).getByRole('button', { name: 'Repost' }));
  await user.click((await screen.findAllByRole('button', { name: 'Repost' }))[1]);

  await waitFor(() => {
    expect(createRepostSpy).toHaveBeenCalledWith(
      'kukuri:topic:demo',
      'kukuri:topic:demo',
      expect.any(String),
      null
    );
  });
  expect(screen.getByText('Reposted')).toBeInTheDocument();
});

test('desktop shell can create a quote repost from the composer', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const originalCreateRepost = api.createRepost;
  const createRepostSpy = vi.fn((topic, sourceTopic, sourceObjectId, commentary) =>
    originalCreateRepost(topic, sourceTopic, sourceObjectId, commentary)
  );
  api.createRepost = createRepostSpy;

  render(<App api={api} />);

  await publishPost(user, 'source post');
  const sourcePost = await screen.findByText('source post');
  const card = sourcePost.closest('article');
  if (!card) {
    throw new Error('source post card not found');
  }

  await user.click(within(card).getByRole('button', { name: 'Repost' }));
  await user.click(await screen.findByRole('button', { name: 'Quote Repost' }));

  const quoteDialog = await screen.findByRole('dialog', { name: 'Quote Repost' });
  const quoteInput = within(quoteDialog).getByPlaceholderText('Write a quote repost');
  expect(within(quoteDialog).getByText('Quote reposting')).toBeInTheDocument();
  expect(within(quoteDialog).getByText('Original post')).toBeInTheDocument();
  expect(within(quoteDialog).getByText('source post')).toBeInTheDocument();
  expect(within(quoteDialog).getByLabelText(/attachment/i)).toBeDisabled();

  await user.type(quoteInput, 'quoted take');
  const composer = quoteInput.closest('form');
  if (!composer) {
    throw new Error('quote repost composer form not found');
  }
  const submitButton = within(composer).getByRole('button', { name: 'Quote Repost' });
  await user.click(submitButton);

  await waitFor(() => {
    expect(createRepostSpy).toHaveBeenCalledWith(
      'kukuri:topic:demo',
      'kukuri:topic:demo',
      expect.any(String),
      'quoted take'
    );
  });
  expect(screen.getByText('quoted take')).toBeInTheDocument();
});

test('reaction popover supports search and recent reactions without legacy management actions', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await publishPost(user, 'reactable post');
  const postCard = (await screen.findByText('reactable post')).closest('article');
  if (!(postCard instanceof HTMLElement)) {
    throw new Error('reactable post card not found');
  }

  await user.click(within(postCard).getByRole('button', { name: 'React' }));
  const searchInput = await screen.findByPlaceholderText('Search reactions');
  expect(screen.queryByRole('button', { name: 'Manage reactions' })).not.toBeInTheDocument();

  await user.type(searchInput, 'party');
  await user.click(screen.getByRole('button', { name: 'party-popper' }));

  await waitFor(() => {
    expect(within(postCard).getByText('🎉')).toBeInTheDocument();
  });

  await user.click(within(postCard).getByRole('button', { name: 'React' }));
  expect(await screen.findByText('Recent')).toBeInTheDocument();
  expect(screen.getByText('Emoji')).toBeInTheDocument();
  expect(screen.getByText('Custom')).toBeInTheDocument();
  expect(
    within(screen.getByText('Recent').closest('section') as HTMLElement).getByRole('button', {
      name: 'party-popper',
    })
  ).toBeInTheDocument();
});

test('reaction picker lazily loads recent and custom reactions when opened', async () => {
  const user = userEvent.setup();
  const baseApi = createDesktopMockApi();
  const api: DesktopApi = {
    ...baseApi,
    listRecentReactions: vi.fn(baseApi.listRecentReactions),
    listMyCustomReactionAssets: vi.fn(baseApi.listMyCustomReactionAssets),
    listBookmarkedCustomReactions: vi.fn(baseApi.listBookmarkedCustomReactions),
  };

  render(<App api={api} />);

  await publishPost(user, 'reaction preload');
  const postCard = (await screen.findByText('reaction preload')).closest('article');
  if (!(postCard instanceof HTMLElement)) {
    throw new Error('reaction preload post card not found');
  }

  expect(api.listRecentReactions).not.toHaveBeenCalled();
  expect(api.listMyCustomReactionAssets).not.toHaveBeenCalled();
  expect(api.listBookmarkedCustomReactions).not.toHaveBeenCalled();

  await user.click(within(postCard).getByRole('button', { name: 'React' }));

  await waitFor(() => {
    expect(api.listRecentReactions).toHaveBeenCalledTimes(1);
    expect(api.listMyCustomReactionAssets).toHaveBeenCalledTimes(1);
    expect(api.listBookmarkedCustomReactions).toHaveBeenCalledTimes(1);
  });
});

test('visible custom reactions auto-fetch media before save, and saved reactions require explicit save', async () => {
  const user = userEvent.setup();
  installObjectUrlMocks();
  const remoteReactionAsset = {
    asset_id: 'asset-remote',
    owner_pubkey: 'd'.repeat(64),
    blob_hash: 'blob-remote',
    search_key: 'remote-cat',
    mime: 'image/png',
    bytes: 128,
    width: 128,
    height: 128,
  };
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-remote-reaction',
          envelope_id: 'envelope-post-remote-reaction',
          author_pubkey: 'f'.repeat(64),
          author_name: 'frank',
          author_display_name: 'Frank',
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'remote custom reaction',
          content_status: 'Available',
          attachments: [],
          created_at: 10,
          reply_to: null,
          root_id: 'post-remote-reaction',
          channel_id: null,
          audience_label: 'Public',
          published_topic_id: 'kukuri:topic:demo',
          origin_topic_id: 'kukuri:topic:demo',
          reaction_summary: [
            {
              reaction_key_kind: 'custom_asset',
              normalized_reaction_key: 'custom_asset:asset-remote',
              emoji: null,
              custom_asset: remoteReactionAsset,
              count: 1,
            },
          ],
          my_reactions: [],
        },
      ],
    },
  });
  const getBlobMediaPayload = vi.fn(async (hash: string, mime: string) =>
    hash === remoteReactionAsset.blob_hash
      ? {
          bytes_base64: 'ZmFrZS1pbWFnZQ==',
          mime,
        }
      : null
  );
  const bookmarkCustomReaction = vi.fn(api.bookmarkCustomReaction.bind(api));
  api.getBlobMediaPayload = getBlobMediaPayload;
  api.bookmarkCustomReaction = bookmarkCustomReaction;

  render(<App api={api} />);

  const remoteReactionImage = await screen.findByAltText(remoteReactionAsset.asset_id);
  expect(remoteReactionImage.getAttribute('src')).toContain('blob:mock-');
  await waitFor(() => {
    expect(getBlobMediaPayload).toHaveBeenCalledWith(
      remoteReactionAsset.blob_hash,
      remoteReactionAsset.mime
    );
  });

  let drawer = await openSettingsSection(user, 'reactions');
  expect(within(drawer).queryByRole('img', { name: remoteReactionAsset.search_key })).toBeNull();

  await user.click(within(drawer).getByRole('button', { name: 'Close settings' }));
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Settings' })).not.toBeInTheDocument();
  });

  const remoteReactionChip = remoteReactionImage.closest('button');
  if (!(remoteReactionChip instanceof HTMLButtonElement)) {
    throw new Error('remote reaction chip not found');
  }

  fireEvent.contextMenu(remoteReactionChip);
  await user.click(screen.getByRole('menuitem', { name: 'Save' }));
  expect(bookmarkCustomReaction).toHaveBeenCalledWith(remoteReactionAsset);

  drawer = await openSettingsSection(user, 'reactions');
  expect(await within(drawer).findByRole('img', { name: remoteReactionAsset.search_key })).toBeInTheDocument();
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
    expect(
      screen.getByRole('button', { name: 'Open core channel settings' })
    ).toBeInTheDocument();
    expect(screen.getByText('channel post')).toBeInTheDocument();
  });
  expect(screen.queryByText('public post')).not.toBeInTheDocument();

  channelTimelineItems = [channelNewPost, channelPost];
  window.dispatchEvent(new Event('focus'));

  expect(await screen.findByRole('button', { name: 'Show 1 new post' })).toBeInTheDocument();
  expect(screen.queryByText('public post')).not.toBeInTheDocument();
  expect(screen.queryByText('channel post new')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Show 1 new post' }));

  await waitFor(() => {
    expect(screen.getByText('channel post new')).toBeInTheDocument();
  });
  expect(screen.queryByText('public post')).not.toBeInTheDocument();

  const topicItem = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
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
  expect(screen.queryByText('channel post')).not.toBeInTheDocument();
  expect(screen.queryByText('channel post new')).not.toBeInTheDocument();
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

test('clicking a timeline post opens thread and author detail flows in the context pane', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
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

  const gameSection = screen.getByText('Game Rooms').closest('.shell-section');
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

test('desktop shell can create, join, leave, and end a live session', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const liveDialog = await openLiveCreateDialog(user);
  await user.type(within(liveDialog).getByPlaceholderText('Friday stream'), 'Launch Party');
  await user.type(within(liveDialog).getByPlaceholderText('short session summary'), 'watch along');
  await user.click(within(liveDialog).getByRole('button', { name: 'Start Live' }));

  await waitFor(() => {
    expect(screen.getByText('Launch Party')).toBeInTheDocument();
  });
  expect(screen.getByText('watch along')).toBeInTheDocument();

  const liveCard = screen.getByText('Launch Party').closest('article');
  if (!(liveCard instanceof HTMLElement)) {
    throw new Error('live session card not found');
  }

  await user.click(within(liveCard).getByRole('button', { name: 'Join' }));
  await waitFor(() => {
    expect(screen.getByText('viewers: 1')).toBeInTheDocument();
  });
  expect(within(liveCard).getByRole('button', { name: 'Leave' })).toBeInTheDocument();

  await user.click(within(liveCard).getByRole('button', { name: 'Leave' }));
  await waitFor(() => {
    expect(screen.getByText('viewers: 0')).toBeInTheDocument();
  });

  await user.click(within(liveCard).getByRole('button', { name: 'End' }));
  await waitFor(() => {
    expect(screen.getByText('Ended')).toBeInTheDocument();
  });
});

test('desktop shell can create a private channel and export an invite', async () => {
  const user = userEvent.setup();
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText,
    },
  });
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(within(channelDialog).getByText('Copy share link')).toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith(
    buildChannelAccessPreviewDeepLink('invite:kukuri:topic:demo:channel-1')
  );
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  const settingsDialog = await openChannelSettings(user, 'core');
  await user.click(getChannelShareButton(settingsDialog, 'core', 'Invite only'));

  await waitFor(() => {
    expect(within(settingsDialog).getByText('Copy share link')).toBeInTheDocument();
    expect(within(settingsDialog).queryByText(/invite:kukuri:topic:demo:channel-1/)).not.toBeInTheDocument();
  });
});

test('desktop shell confirms and leaves a private channel', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi();
  const leavePrivateChannel = vi.spyOn(api, 'leavePrivateChannel');
  render(<App api={api} />);

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  await user.click(screen.getByRole('button', { name: 'Leave core channel' }));
  let leaveDialog = await screen.findByRole('dialog', { name: 'Leave channel' });
  expect(within(leaveDialog).getByText('Leave this channel?')).toBeInTheDocument();
  await user.click(within(leaveDialog).getByRole('button', { name: 'Close dialog' }));
  expect(leavePrivateChannel).not.toHaveBeenCalled();

  await user.click(screen.getByRole('button', { name: 'Leave core channel' }));
  leaveDialog = await screen.findByRole('dialog', { name: 'Leave channel' });
  await user.click(within(leaveDialog).getByRole('button', { name: 'No' }));
  expect(leavePrivateChannel).not.toHaveBeenCalled();

  await user.click(screen.getByRole('button', { name: 'Leave core channel' }));
  leaveDialog = await screen.findByRole('dialog', { name: 'Leave channel' });
  await user.click(within(leaveDialog).getByRole('button', { name: 'Yes' }));

  await waitFor(() => {
    expect(leavePrivateChannel).toHaveBeenCalledWith('kukuri:topic:demo', 'channel-1');
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
    expect(screen.queryByRole('button', { name: /core.*Invite only/ })).not.toBeInTheDocument();
  });
});

test('desktop shell joins an imported private channel and selects its topic scope', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createDesktopMockApi({
        invitePreview: {
          channel_id: 'channel-imported',
          topic_id: 'kukuri:topic:private-imported',
          channel_label: 'Imported',
          inviter_pubkey: 'f'.repeat(64),
          owner_pubkey: 'f'.repeat(64),
          epoch_id: 'epoch-imported-1',
          expires_at: null,
          namespace_secret_hex: 'a'.repeat(64),
        },
      })}
    />
  );
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText(/paste private channel invite/i), 'invite-token');
  await user.click(within(channelDialog).getByRole('button', { name: 'Join' }));
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  await waitFor(() => {
    expectActiveTopic('kukuri:topic:private-imported');
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Aprivate-imported&channel=channel-imported'
    );
  });
});

test('channel route restore waits for joined channel list before normalizing', async () => {
  const joinedChannels = createDeferred<JoinedPrivateChannelView[]>();
  const api = createDesktopMockApi();
  const listJoinedPrivateChannels = vi
    .spyOn(api, 'listJoinedPrivateChannels')
    .mockImplementation(async (topic) => {
      if (topic !== 'kukuri:topic:demo') {
        return [];
      }
      return joinedChannels.promise;
    });

  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-restored', api);

  await waitFor(() => {
    expect(listJoinedPrivateChannels).toHaveBeenCalledWith('kukuri:topic:demo');
  });
  expect(window.location.hash).toBe(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-restored'
  );

  joinedChannels.resolve([
    {
      topic_id: 'kukuri:topic:demo',
      channel_id: 'channel-restored',
      label: 'restored',
      creator_pubkey: 'f'.repeat(64),
      owner_pubkey: 'f'.repeat(64),
      joined_via_pubkey: null,
      audience_kind: 'friend_plus',
      is_owner: false,
      current_epoch_id: 'epoch-restored',
      archived_epoch_ids: [],
      sharing_state: 'open',
      rotation_required: false,
      participant_count: 1,
      stale_participant_count: 0,
    },
  ]);

  await waitFor(() => {
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-restored'
    );
    expect(screen.getByRole('button', { name: /restored.*Friends\+/ })).toHaveClass(
      'topic-subitem-active'
    );
  });
});

test('desktop shell shows friend-only controls and can create a grant', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'friends');
  await user.selectOptions(within(channelDialog).getByLabelText('Audience'), 'friend_only');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(screen.queryByRole('button', { name: 'Rotate' })).not.toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  const settingsDialog = await openChannelSettings(user, 'friends');
  await user.click(getChannelShareButton(settingsDialog, 'friends', 'Friends'));

  await waitFor(() => {
    expect(within(settingsDialog).getByText('Copy share link')).toBeInTheDocument();
    expect(within(settingsDialog).queryByText(/grant:kukuri:topic:demo:channel-1/)).not.toBeInTheDocument();
  });
});

test('desktop shell shows friend-plus controls and can create a share', async () => {
  const user = userEvent.setup();
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText,
    },
  });
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'friends+');
  await user.selectOptions(within(channelDialog).getByLabelText('Audience'), 'friend_plus');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(screen.queryByRole('button', { name: 'Freeze' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rotate' })).not.toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  const settingsDialog = await openChannelSettings(user, 'friends+');
  await user.click(getChannelShareButton(settingsDialog, 'friends+', 'Friends+'));

  await waitFor(() => {
    expect(within(settingsDialog).getByText('Copy share link')).toBeInTheDocument();
    expect(within(settingsDialog).queryByText(/share:kukuri:topic:demo:channel-1/)).not.toBeInTheDocument();
  });

  await user.click(within(settingsDialog).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith(
    buildChannelAccessPreviewDeepLink('share:kukuri:topic:demo:channel-1')
  );
});

test('share token smart link previews before import and joins only after confirmation', async () => {
  const user = userEvent.setup();
  const ownerPubkey = 'f'.repeat(64);
  const inviteToken = JSON.stringify({
    envelope: {
      kind: 'channel-invite',
      pubkey: ownerPubkey,
      content: JSON.stringify({
        channel_id: 'channel-imported',
        topic_id: 'kukuri:topic:private-imported',
        channel_label: 'Imported',
        owner_pubkey: ownerPubkey,
        epoch_id: 'epoch-imported-1',
        namespace_secret_hex: 'a'.repeat(64),
        expires_at: null,
      }),
    },
  });
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'share-post',
          envelope_id: 'envelope-share-post',
          author_pubkey: 'a'.repeat(64),
          author_name: 'alice',
          author_display_name: null,
          following: false,
          followed_by: false,
          mutual: false,
          friend_of_friend: false,
          object_kind: 'post',
          content: buildChannelAccessPreviewDeepLink(inviteToken),
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'share-post',
          channel_id: null,
          audience_label: 'Public',
        },
      ],
    },
    invitePreview: {
      channel_id: 'channel-imported',
      topic_id: 'kukuri:topic:private-imported',
      channel_label: 'Imported',
      inviter_pubkey: ownerPubkey,
      owner_pubkey: ownerPubkey,
      epoch_id: 'epoch-imported-1',
      expires_at: null,
      namespace_secret_hex: 'a'.repeat(64),
    },
  });
  const previewSpy = vi.spyOn(api, 'previewChannelAccessToken');
  const importSpy = vi.spyOn(api, 'importChannelAccessToken');

  render(<App api={api} />);

  const tokenChip = (await screen.findAllByRole('button', { name: /Imported.*Invite only/ }))
    .find((button) => button.classList.contains('smart-reference-chip'));
  if (!(tokenChip instanceof HTMLButtonElement)) {
    throw new Error('expected access preview chip');
  }
  expect(tokenChip).not.toHaveAttribute('title');
  await user.hover(tokenChip);
  expect(await screen.findByRole('tooltip')).toHaveTextContent(inviteToken);
  await user.unhover(tokenChip);

  await user.click(tokenChip);

  const dialog = await screen.findByRole('dialog', { name: 'Preview Access' });
  await waitFor(() => {
    expect(previewSpy).toHaveBeenCalledTimes(1);
    expect(previewSpy).toHaveBeenCalledWith(inviteToken);
    expect(importSpy).not.toHaveBeenCalled();
  });
  expect(within(dialog).getByText('Imported')).toBeInTheDocument();
  expect(within(dialog).queryByText(/channel-imported/)).not.toBeInTheDocument();
  expect(within(dialog).queryByText(/epoch-imported-1/)).not.toBeInTheDocument();
  const channelPreviewItem = within(dialog).getByText('Imported').closest('div');
  if (!(channelPreviewItem instanceof HTMLElement)) {
    throw new Error('expected channel preview item');
  }
  expect(channelPreviewItem).not.toHaveAttribute('title');
  await user.hover(channelPreviewItem);
  expect(await screen.findByRole('tooltip')).toHaveTextContent('channel-imported');

  await user.click(within(dialog).getByRole('button', { name: 'Import / Join' }));

  await waitFor(() => {
    expect(importSpy).toHaveBeenCalledTimes(1);
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Aprivate-imported&channel=channel-imported'
    );
  });
});

test('copy link actions write canonical hash routes for topic, post, live, and game', async () => {
  const user = userEvent.setup();
  const writeText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(window.navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText,
    },
  });

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            {
              object_id: 'copy-post',
              envelope_id: 'envelope-copy-post',
              author_pubkey: 'a'.repeat(64),
              author_name: 'alice',
              author_display_name: null,
              following: false,
              followed_by: false,
              mutual: false,
              friend_of_friend: false,
              object_kind: 'post',
              content: 'copy this post',
              content_status: 'Available',
              attachments: [],
              created_at: 1,
              reply_to: null,
              root_id: 'copy-post',
              channel_id: null,
              audience_label: 'Public',
            },
          ],
        },
        seedLiveSessions: {
          'kukuri:topic:demo': [
            {
              session_id: 'session-demo',
              host_pubkey: 'a'.repeat(64),
              title: 'Live Demo',
              description: 'watch here',
              status: 'Live',
              started_at: 1,
              viewer_count: 1,
              joined_by_me: false,
              channel_id: null,
              audience_label: 'Public',
            },
          ],
        },
        seedGameRooms: {
          'kukuri:topic:demo': [
            {
              room_id: 'room-demo',
              host_pubkey: 'a'.repeat(64),
              title: 'Room Demo',
              description: 'play here',
              status: 'Waiting',
              phase_label: 'Round 1',
              scores: [],
              updated_at: 1,
              channel_id: null,
              audience_label: 'Public',
            },
          ],
        },
      })}
    />
  );

  const topicItem = screen.getByRole('button', { name: 'kukuri:topic:demo' }).closest('li');
  if (!(topicItem instanceof HTMLElement)) {
    throw new Error('expected topic item');
  }
  await user.click(within(topicItem).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith('#/timeline?topic=kukuri%3Atopic%3Ademo');
  await waitFor(() => {
    expect(screen.getByRole('status')).toHaveTextContent('Copied to clipboard.');
    expect(screen.getAllByRole('status')).toHaveLength(1);
  });

  const postArticle = screen.getByText('copy this post').closest('article');
  if (!(postArticle instanceof HTMLElement)) {
    throw new Error('expected post article');
  }
  await user.click(within(postArticle).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=copy-post&focusObjectId=copy-post'
  );
  await waitFor(() => {
    expect(screen.getAllByRole('status')).toHaveLength(1);
  });

  await selectWorkspace(user, 'Live');
  const liveArticle = screen.getByText('Live Demo').closest('article');
  if (!(liveArticle instanceof HTMLElement)) {
    throw new Error('expected live article');
  }
  await user.click(within(liveArticle).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith(
    '#/live?topic=kukuri%3Atopic%3Ademo&sessionId=session-demo'
  );
  await waitFor(() => {
    expect(screen.getAllByRole('status')).toHaveLength(1);
  });

  await selectWorkspace(user, 'Game');
  const gameArticle = screen.getByText('Room Demo').closest('article');
  if (!(gameArticle instanceof HTMLElement)) {
    throw new Error('expected game article');
  }
  await user.click(within(gameArticle).getByRole('button', { name: 'Copy link' }));
  expect(writeText).toHaveBeenLastCalledWith(
    '#/game?topic=kukuri%3Atopic%3Ademo&roomId=room-demo'
  );
  await waitFor(() => {
    expect(screen.getAllByRole('status')).toHaveLength(1);
  });
});

test('channel settings copy removes duplicate summary and share button icon', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('Channel name'), 'friends+');
  await user.selectOptions(within(channelDialog).getByLabelText('Audience'), 'friend_plus');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
  });
  expect(
    within(channelDialog).queryByRole('button', { name: 'Create share link' })
  ).not.toBeInTheDocument();
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  const settingsDialog = await openChannelSettings(user, 'friends+');
  const shareButton = await within(settingsDialog).findByRole('button', {
    name: 'Create share link',
  });
  expect(within(settingsDialog).getByText('Channel name: friends+')).toBeInTheDocument();
  expect(
    within(settingsDialog).getByText('Policy: Friends+: participants can share to their mutuals')
  ).toBeInTheDocument();
  expect(within(settingsDialog).queryByText('friends+ / Friends+')).not.toBeInTheDocument();
  expect(shareButton).toHaveTextContent('Create share link');
  expect(shareButton.querySelector('svg')).not.toBeInTheDocument();
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
      'kukuri:topic:demo': threadPosts,
    },
  });
  api.listThread = vi.fn(async (_topic: string, threadId: string, cursor: TimelineCursor | null, limit = 30) => {
    if (threadId !== 'paginated-post-1') {
      return { items: [], next_cursor: null };
    }
    return paginatePosts(threadPosts, cursor, limit);
  });

  renderAtHash(
    '#/timeline?topic=kukuri%3Atopic%3Ademo&context=thread&threadId=paginated-post-1&focusObjectId=paginated-post-11',
    api
  );

  await waitFor(() => {
    expect(getDetailPane('Thread')).toBeInTheDocument();
  });
  await waitFor(() => {
    expect(scrollIntoView).toHaveBeenCalledTimes(1);
  });

  await user.click(within(getDetailPane('Thread')).getByRole('button', { name: 'Load more' }));

  await waitFor(() => {
    expect(within(getDetailPane('Thread')).getByText('paginated post 1')).toBeInTheDocument();
  });
  expect(scrollIntoView).toHaveBeenCalledTimes(1);
});

test('desktop shell can create and update a game room', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const gameDialog = await openGameCreateDialog(user);
  await user.type(within(gameDialog).getByPlaceholderText('Top 8 Finals'), 'Grand Finals');
  await user.type(within(gameDialog).getByPlaceholderText('match summary'), 'set one');
  await user.type(within(gameDialog).getByPlaceholderText('Alice, Bob'), 'Alice, Bob');
  await user.click(within(gameDialog).getByRole('button', { name: 'Create Room' }));

  await waitFor(() => {
    expect(screen.getByText('Grand Finals')).toBeInTheDocument();
    expect(screen.getByLabelText(/game-.*-status/)).toBeInTheDocument();
  });
  expect(screen.getByText('set one')).toBeInTheDocument();
  expect(screen.getByLabelText(/game-.*-Alice-score/)).toBeInTheDocument();

  await user.selectOptions(screen.getByLabelText(/game-.*-status/), 'Running');
  await user.clear(screen.getByLabelText(/game-.*-phase/));
  await user.type(screen.getByLabelText(/game-.*-phase/), 'Round 3');
  await user.clear(screen.getByLabelText(/game-.*-Alice-score/));
  await user.type(screen.getByLabelText(/game-.*-Alice-score/), '2');
  await user.click(screen.getByRole('button', { name: 'Save Room' }));

  await waitFor(() => {
    expect(screen.getByLabelText(/game-.*-status/)).toHaveValue('Running');
  });
  expect(screen.getByText('phase: Round 3')).toBeInTheDocument();
  expect(screen.getByDisplayValue('2')).toBeInTheDocument();
});

test('single attach button classifies mixed image and video files', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(within(publishDialog).getByLabelText(/attachment/i), [
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' }),
    new File([Uint8Array.from([5, 6, 7, 8])], 'clip.mp4', { type: 'video/mp4' }),
  ]);
  await waitFor(() => {
    expect(screen.getByText('flower.png')).toBeInTheDocument();
    expect(screen.getByText('clip.mp4')).toBeInTheDocument();
  });
  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(3);
  });
  expect(attachmentsSeen.map((attachment) => attachment.role)).toEqual([
    'image_original',
    'video_manifest',
    'video_poster',
  ]);
});

test('video upload generates poster attachment before publish', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  });
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();

  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(2);
  });
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_manifest')).toBe(true);
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_poster')).toBe(true);
});

test('video upload generates poster attachment with metadata seek fallback', async () => {
  installObjectUrlMocks();
  installMetadataSeekPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createDesktopMockApi();
  const originalCreatePost = api.createPost;
  api.createPost = async (topic, content, replyTo, attachments) => {
    attachmentsSeen = attachments ?? [];
    return originalCreatePost(topic, content, replyTo, attachments);
  };

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  });
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();

  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(attachmentsSeen).toHaveLength(2);
  });
  expect(attachmentsSeen.some((attachment) => attachment.role === 'video_poster')).toBe(true);
});

test('video poster generation failure blocks publish', async () => {
  installObjectUrlMocks();
  installFailedPosterGenerationMocks();
  const api = createDesktopMockApi();
  const createPostSpy = vi.fn(api.createPost);
  api.createPost = createPostSpy;

  const user = userEvent.setup();
  render(<App api={api} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([1, 3, 5, 7])], 'broken.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getAllByText('failed to generate video poster').length).toBeGreaterThan(0);
  });

  await user.click(within(publishDialog).getByRole('button', { name: 'Publish' }));

  expect(createPostSpy).not.toHaveBeenCalled();
});

test('composer shows image draft preview before publish', async () => {
  installObjectUrlMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' })
  );

  expect(await within(publishDialog).findByRole('img', { name: 'draft preview flower.png' })).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image_original/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image\/png/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/4 B/)).toBeInTheDocument();
});

test('composer shows video poster draft preview before publish', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const publishDialog = await openPublishDialog(user);
  await user.upload(
    within(publishDialog).getByLabelText(/attachment/i),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  expect(await within(publishDialog).findByRole('img', { name: 'draft preview clip.mp4' })).toBeInTheDocument();
  expect(within(publishDialog).getByText(/video_manifest/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/video_poster/)).toBeInTheDocument();
  expect(within(publishDialog).getByText(/image\/jpeg/)).toBeInTheDocument();
});

test('timeline image post shows media skeleton when attachment is missing', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [buildImagePost()],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(<App api={api} />);

  await waitFor(() => {
    expect(screen.getByTestId('media-skeleton-image-post')).toBeInTheDocument();
  });
  expect(screen.getByTestId('media-skeleton-image-post')).toBeInTheDocument();
  expect(screen.getByText('image/png')).toBeInTheDocument();
});

test('timeline image post switches to ready state when attachment becomes available', async () => {
  const missingPost = buildImagePost();
  const { rerender } = render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [missingPost],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByTestId('media-skeleton-image-post')).toBeInTheDocument();
  });

  rerender(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
            buildImagePost({
              content: 'caption ready',
              content_status: 'Available' satisfies BlobViewStatus,
              attachments: [
                {
                  ...missingPost.attachments[0],
                  status: 'Available',
                },
              ],
            }),
          ],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByText('caption ready')).toBeInTheDocument();
  });
  expect(screen.queryByTestId('media-skeleton-image-post')).not.toBeInTheDocument();
});

test('timeline image post renders actual preview when object-url payload is available', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildImagePost({
          content: 'caption ready',
          content_status: 'Available',
          attachments: [
            {
              hash: 'b'.repeat(64),
              mime: 'image/png',
              bytes: 4096,
              role: 'image_original',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async () => ({
    bytes_base64: 'ZmFrZS1pbWFnZQ==',
    mime: 'image/png',
  });

  render(<App api={api} />);

  const preview = await screen.findByTestId('media-preview-image-post');
  expect(preview).toBeInTheDocument();
  expect(preview.getAttribute('src')).toContain('blob:mock-');
});

test('thread pane reuses the same image placeholder renderer', async () => {
  const user = userEvent.setup();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildImagePost(),
        {
          ...buildImagePost({
            object_id: 'reply-post',
            envelope_id: 'envelope-reply-post',
            object_kind: 'comment',
            content: 'reply body',
            content_status: 'Available',
            attachments: [],
            reply_to: 'image-post',
            root_id: 'image-post',
          }),
        },
      ],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(
    <App api={api} />
  );

  await waitFor(() => {
    expect(screen.getByText('envelope-image-post')).toBeInTheDocument();
  });
  await user.click(screen.getByText('envelope-image-post'));
  const threadPanel = await screen.findByRole('complementary', { name: 'Thread' });

  await waitFor(() => {
    expect(within(threadPanel).getByTestId('media-skeleton-image-post')).toBeInTheDocument();
  });
  expect(within(threadPanel).getByTestId('media-skeleton-image-post')).toBeInTheDocument();
});

test('text body pending uses text skeleton without hiding image metadata', async () => {
  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [buildImagePost()],
        },
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByTestId('text-skeleton-image-post')).toBeInTheDocument();
  });
  expect(screen.getByText('image/png')).toBeInTheDocument();
  expect(screen.getByText('2.0 KB')).toBeInTheDocument();
});

test('timeline video post shows poster skeleton when poster is missing', async () => {
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [buildVideoPost()],
    },
  });
  api.getBlobMediaPayload = async () => null;

  render(<App api={api} />);

  await waitFor(() => {
    expect(screen.getByTestId('media-skeleton-video-post')).toBeInTheDocument();
  });
  expect(screen.getByTestId('media-skeleton-video-post')).toBeInTheDocument();
  expect(screen.getByText('video/mp4')).toBeInTheDocument();
});

test('poster-only video card renders poster preview without video element', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildVideoPost({
          attachments: [
            {
              hash: 'v'.repeat(64),
              mime: 'video/mp4',
              bytes: 8192,
              role: 'video_manifest',
              status: 'Missing',
            },
            {
              hash: 'p'.repeat(64),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) =>
    hash === 'p'.repeat(64)
      ? {
          bytes_base64: 'ZmFrZS1wb3N0ZXI=',
          mime,
        }
      : null;

  render(<App api={api} />);

  const posterPreview = await screen.findByTestId('media-preview-video-post');
  expect(posterPreview).toBeInTheDocument();
  expect(screen.queryByTestId('media-video-video-post')).not.toBeInTheDocument();
  expect(posterPreview.getAttribute('src')).toContain('blob:mock-');
});

test('video card fetches manifest payload even when attachment status is missing', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildVideoPost({
          attachments: [
            {
              hash: 'late-manifest'.repeat(4),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Missing',
            },
            {
              hash: 'late-poster'.repeat(4),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'late-manifest'.repeat(4)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'late-poster'.repeat(4)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video).toHaveAttribute('src', expect.stringContaining('blob:mock-'));
});

test('video card retries after stalled manifest fetch after rerender', async () => {
  installObjectUrlMocks();
  const manifestHash = 'retry-manifest'.repeat(4);
  const posterHash = 'retry-poster'.repeat(4);
  const seedPosts = {
    'kukuri:topic:demo': [
      buildVideoPost({
        attachments: [
          {
            hash: manifestHash,
            mime: 'video/mp4',
            bytes: 9999,
            role: 'video_manifest',
            status: 'Missing',
          },
          {
            hash: posterHash,
            mime: 'image/jpeg',
            bytes: 1024,
            role: 'video_poster',
            status: 'Missing',
          },
        ],
      }),
    ],
  };
  const stalledApi = createDesktopMockApi({
    seedPosts,
  });
  stalledApi.getBlobMediaPayload = async (hash, mime) => {
    if (hash === manifestHash) {
      return new Promise<null>(() => {});
    }
    if (hash === posterHash) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };
  const recoveredApi = createDesktopMockApi({
    seedPosts: {
      ...seedPosts,
    },
  });
  recoveredApi.getBlobMediaPayload = async (hash, mime) => {
    if (hash === manifestHash) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === posterHash) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  const { rerender } = render(<App api={stalledApi} />);

  await waitFor(() => {
    expect(screen.getByTestId('media-preview-video-post')).toBeInTheDocument();
  });

  rerender(<App api={recoveredApi} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video).toHaveAttribute('src', expect.stringContaining('blob:mock-'));
});

test('video card renders object-url playback source when manifest payload is available', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildVideoPost({
          attachments: [
            {
              hash: 'manifest'.repeat(8),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Available',
            },
            {
              hash: 'poster'.repeat(8),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'manifest'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'poster'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(video.getAttribute('src')).toContain('blob:mock-');
});

test('video card falls back to poster preview when playback is unsupported on this client', async () => {
  installObjectUrlMocks();
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildVideoPost({
          attachments: [
            {
              hash: 'manifest'.repeat(8),
              mime: 'video/mp4',
              bytes: 9999,
              role: 'video_manifest',
              status: 'Available',
            },
            {
              hash: 'poster'.repeat(8),
              mime: 'image/jpeg',
              bytes: 1024,
              role: 'video_poster',
              status: 'Available',
            },
          ],
        }),
      ],
    },
  });
  api.getBlobMediaPayload = async (hash, mime) => {
    if (hash === 'manifest'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS12aWRlbw==',
        mime,
      };
    }
    if (hash === 'poster'.repeat(8)) {
      return {
        bytes_base64: 'ZmFrZS1wb3N0ZXI=',
        mime,
      };
    }
    return null;
  };

  render(<App api={api} />);

  const video = await screen.findByTestId('media-video-video-post');
  Object.defineProperty(video, 'error', {
    configurable: true,
    get: () => ({ code: 4 }),
  });
  fireEvent.error(video);

  await waitFor(() => {
    expect(screen.queryByTestId('media-video-video-post')).not.toBeInTheDocument();
  });
  expect(screen.getByTestId('media-preview-video-post')).toBeInTheDocument();
  expect(screen.getByAltText('video poster')).toBeInTheDocument();
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

test('timeline author detail opens as a single pane, and thread author detail stacks to the right', async () => {
  const user = userEvent.setup();
  const authorPubkey = 'a'.repeat(64);
  const createApi = () =>
    createDesktopMockApi({
      seedPosts: {
        'kukuri:topic:demo': [
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

  const { unmount } = renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo', createApi());

  await user.click(await screen.findByRole('button', { name: 'alice' }));
  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(screen.queryByRole('complementary', { name: 'Thread' })).not.toBeInTheDocument();

  unmount();
  renderAtHash('#/timeline?topic=kukuri%3Atopic%3Ademo', createApi());

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
  expect(screen.getByText('Visible Room')).toBeInTheDocument();
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

test('author detail mutual action opens the messages workspace and sends a local message', async () => {
  const authorPubkey = 'b'.repeat(64);
  const api = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-author-dm',
          envelope_id: 'envelope-author-dm',
          author_pubkey: authorPubkey,
          author_name: 'bob',
          author_display_name: null,
          following: true,
          followed_by: true,
          mutual: true,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'open dm',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-author-dm',
          audience_label: 'Public',
        },
      ],
    },
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
  const user = userEvent.setup();

  render(<App api={api} />);

  await user.click(await screen.findByRole('button', { name: 'bob' }));
  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Message' }));

  await waitFor(() => {
    expect(within(getWorkspaceTabs()).getByRole('tab', { name: 'Messages' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(window.location.hash).toBe(
      `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}`
    );
  });

  await waitFor(() => {
    expect(screen.getByPlaceholderText('Write a message')).not.toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send' })).not.toBeDisabled();
  });
  fireEvent.change(screen.getByPlaceholderText('Write a message'), {
    target: { value: 'hello dm' },
  });
  await user.click(screen.getByRole('button', { name: 'Send' }));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
  }, { timeout: 3000 });
});

test('messages conversation list rows render avatars', async () => {
  installObjectUrlMocks();

  const authorPubkey = 'b'.repeat(64);
  const api = createDesktopMockApi({
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
        picture_asset: {
          hash: 'dm-conversation-avatar',
          mime: 'image/png',
          bytes: 64,
          role: 'profile_avatar',
        },
      },
    },
  });
  await api.openDirectMessage(authorPubkey);

  renderAtHash('#/messages?topic=kukuri%3Atopic%3Ademo', api);

  const avatar = await screen.findByTestId(`dm-conversation-avatar-${authorPubkey}`);
  await waitFor(() => {
    expect(avatar.querySelector('img')?.getAttribute('src')).toBe('blob:mock-1');
  });
});

test('messages author click opens the author pane without leaving the selected dm', async () => {
  const authorPubkey = 'b'.repeat(64);
  const api = createDesktopMockApi({
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
  await api.sendDirectMessage(authorPubkey, 'hello dm');
  const user = userEvent.setup();

  renderAtHash(
    `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}`,
    api
  );

  const conversationAvatar = await screen.findByTestId(`dm-conversation-avatar-${authorPubkey}`);
  await waitFor(() => {
    expect(window.location.hash).toBe(
      `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}`
    );
  });
  const conversationIdentity = conversationAvatar.closest('.post-meta-author');
  if (!(conversationIdentity instanceof HTMLElement)) {
    throw new Error('dm conversation author identity not found');
  }
  await user.click(conversationIdentity);

  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
    expect(within(getWorkspaceTabs()).getByRole('tab', { name: 'Messages' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(window.location.hash).toBe(
      `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}&authorPubkey=${authorPubkey}`
    );
  });
});

test('messages dm headers use resolved author labels instead of You and Peer', async () => {
  const authorPubkey = 'b'.repeat(64);
  const baseApi = createDesktopMockApi({
    myProfile: {
      display_name: 'Local Author',
    },
    authorSocialViews: {
      [authorPubkey]: {
        display_name: 'Bob Display',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
  const localAuthorPubkey = (await baseApi.getSyncStatus()).local_author_pubkey;
  const conversation = await baseApi.openDirectMessage(authorPubkey);
  await baseApi.sendDirectMessage(authorPubkey, 'hello dm');
  const api: DesktopApi = {
    ...baseApi,
    async listDirectMessageMessages(pubkey, cursor, limit) {
      const timeline = await baseApi.listDirectMessageMessages(pubkey, cursor, limit);
      const incomingMessage: DirectMessageMessageView = {
        dm_id: conversation.dm_id,
        message_id: 'dm-incoming-1',
        sender_pubkey: authorPubkey,
        recipient_pubkey: localAuthorPubkey,
        created_at: 2,
        text: 'reply from bob',
        reply_to_message_id: null,
        attachments: [],
        outgoing: false,
        delivered: true,
      };
      return {
        items: [incomingMessage, ...timeline.items],
        next_cursor: null,
      };
    },
  };

  renderAtHash(
    `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}`,
    api
  );

  await screen.findByText('hello dm');
  await waitFor(() => {
    expect(screen.getAllByRole('button', { name: 'Local Author' }).length).toBeGreaterThan(0);
    expect(screen.getAllByRole('button', { name: 'Bob Display' }).length).toBeGreaterThan(0);
  });
  expect(screen.queryByText('You')).not.toBeInTheDocument();
  expect(screen.queryByText('Peer')).not.toBeInTheDocument();
});

test('messages hash route restores the direct message and author pane together', async () => {
  const authorPubkey = 'b'.repeat(64);
  const api = createDesktopMockApi({
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });

  renderAtHash(
    `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}&authorPubkey=${authorPubkey}`,
    api
  );

  await waitFor(() => {
    expect(within(getWorkspaceTabs()).getByRole('tab', { name: 'Messages' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  expect(screen.getByPlaceholderText('Write a message')).toBeInTheDocument();
  expect(window.location.hash).toBe(
    `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}&authorPubkey=${authorPubkey}`
  );
});

test('switching messages peer closes a stale author pane', async () => {
  const firstAuthorPubkey = 'b'.repeat(64);
  const secondAuthorPubkey = 'c'.repeat(64);
  const api = createDesktopMockApi({
    authorSocialViews: {
      [firstAuthorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
      },
      [secondAuthorPubkey]: {
        name: 'carol',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
  await api.openDirectMessage(firstAuthorPubkey);
  await api.openDirectMessage(secondAuthorPubkey);
  const user = userEvent.setup();

  renderAtHash(
    `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${firstAuthorPubkey}&authorPubkey=${firstAuthorPubkey}`,
    api
  );

  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });

  const secondConversationCard = screen.getByText('carol').closest('article');
  if (!(secondConversationCard instanceof HTMLElement)) {
    throw new Error('second conversation card not found');
  }
  await user.click(within(secondConversationCard).getByRole('button', { name: 'Open' }));

  await waitFor(() => {
    expect(screen.queryByRole('complementary', { name: 'Author' })).not.toBeInTheDocument();
    expect(window.location.hash).toBe(
      `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${secondAuthorPubkey}`
    );
  });
});

test('messages workspace keeps the last successful DM state when status refresh fails', async () => {
  const authorPubkey = 'b'.repeat(64);
  let failNextStatusRefresh = false;
  const baseApi = createDesktopMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        {
          object_id: 'post-author-dm-refresh',
          envelope_id: 'envelope-author-dm-refresh',
          author_pubkey: authorPubkey,
          author_name: 'bob',
          author_display_name: null,
          following: true,
          followed_by: true,
          mutual: true,
          friend_of_friend: false,
          object_kind: 'post',
          content: 'open dm refresh',
          content_status: 'Available',
          attachments: [],
          created_at: 1,
          reply_to: null,
          root_id: 'post-author-dm-refresh',
          audience_label: 'Public',
        },
      ],
    },
    authorSocialViews: {
      [authorPubkey]: {
        name: 'bob',
        following: true,
        followed_by: true,
        mutual: true,
      },
    },
  });
  const api: DesktopApi = {
    ...baseApi,
    async getDirectMessageStatus(pubkey) {
      if (failNextStatusRefresh) {
        failNextStatusRefresh = false;
        throw new Error('temporary dm status failure');
      }
      return baseApi.getDirectMessageStatus(pubkey);
    },
  };
  const user = userEvent.setup();

  render(<App api={api} />);

  await user.click(await screen.findByRole('button', { name: 'bob' }));
  await waitFor(() => {
    expect(getDetailPane('Author')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Message' }));
  await waitFor(() => {
    expect(within(getWorkspaceTabs()).getByRole('tab', { name: 'Messages' })).toHaveAttribute(
      'aria-selected',
      'true'
    );
  });
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Write a message')).not.toBeDisabled();
    expect(screen.getByRole('button', { name: 'Send' })).not.toBeDisabled();
  });
  fireEvent.change(screen.getByPlaceholderText('Write a message'), {
    target: { value: 'hello dm' },
  });
  await user.click(screen.getByRole('button', { name: 'Send' }));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
  });

  failNextStatusRefresh = true;
  await new Promise((resolve) => window.setTimeout(resolve, REFRESH_INTERVAL_MS + 300));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
    expect(screen.getByText('temporary dm status failure')).toBeInTheDocument();
    expect(
      screen.queryByText('Direct message send is disabled until the relationship is mutual again.')
    ).not.toBeInTheDocument();
  });
});

test('workspace shows a community-node unavailable notice when a configured node reports an error', async () => {
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

  const notice = await screen.findByTestId('community-node-unavailable-notice');
  expect(notice).toHaveTextContent('Community node unavailable');
  expect(notice).toHaveTextContent(
    'A configured community node is currently unavailable. Startup continues, and direct P2P connections remain available.'
  );
  expect(
    within(notice).getByRole('button', { name: 'Open Community Node Settings' })
  ).toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: 'kukuri:topic:demo' })).toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: 'kukuri:topic:demo' })).toBeInTheDocument();
  });
});

test('author avatar blob stays visible on the timeline after the author pane closes', async () => {
  installObjectUrlMocks();

  const authorPubkey = 'b'.repeat(64);
  const user = userEvent.setup();

  render(
    <App
      api={createDesktopMockApi({
        seedPosts: {
          'kukuri:topic:demo': [
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
  await user.click(within(authorPane).getByRole('button', { name: 'Close Author' }));

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
          'kukuri:topic:demo': [
            {
              object_id: 'post-inline-avatar',
              envelope_id: 'envelope-inline-avatar',
              author_pubkey: authorPubkey,
              author_name: 'carol',
              author_display_name: null,
              author_picture: null,
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
    ).toBe('blob:mock-1');
  });
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
