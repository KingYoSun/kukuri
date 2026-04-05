import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { DESKTOP_THEME_STORAGE_KEY } from '@/lib/theme';
import { App } from '@/App';
import type {
  AttachmentView,
  BlobViewStatus,
  CreateAttachmentInput,
  DesktopApi,
  DirectMessageMessageView,
  PostView,
} from '@/lib/api';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

function setViewportWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: width,
  });
  window.dispatchEvent(new Event('resize'));
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
  return await screen.findByRole('dialog', { name: 'Settings & diagnostics' });
}

async function openSettingsSection(
  user: ReturnType<typeof userEvent.setup>,
  section: 'appearance' | 'connectivity' | 'discovery' | 'community-node'
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

function expectActiveTopicBar(topic: string) {
  expect(screen.getByRole('banner', { name: 'Active topic bar' })).toHaveTextContent(topic);
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
  return await screen.findByRole('dialog', { name: 'Private Channels' });
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
  expectActiveTopicBar('kukuri:topic:demo');
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
  expect(within(dialog).getByPlaceholderText('core contributors')).toBeInTheDocument();

  await user.click(within(dialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(screen.queryByRole('dialog', { name: 'Private Channels' })).not.toBeInTheDocument();
  });
});

test('invalid hash routes fall back to the active public timeline and normalize the URL', async () => {
  renderAtHash(
    '#/unknown?topic=missing-topic&timelineScope=channel:missing&composeTarget=channel:missing&context=author&authorPubkey=bad&settings=invalid'
  );

  await waitFor(() => {
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo');
  });
  expectActiveTopicBar('kukuri:topic:demo');
  expect(
    screen.queryByRole('dialog', { name: 'Settings & diagnostics' })
  ).not.toBeInTheDocument();
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

  const drawer = await screen.findByRole('dialog', { name: 'Settings & diagnostics' });
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

  const drawer = await screen.findByRole('dialog', { name: 'Settings & diagnostics' });
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
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'core');
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
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Private Channels' })
    ).not.toBeInTheDocument();
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
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
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
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'second-core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  await waitFor(() => {
    expectActiveTopicBar('kukuri:topic:demo');
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
    expectActiveTopicBar('kukuri:topic:second');
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

  await user.type(searchInput, '🎉');
  await user.click(screen.getByRole('button', { name: '🎉' }));

  await waitFor(() => {
    expect(within(postCard).getByText('🎉')).toBeInTheDocument();
  });

  await user.click(within(postCard).getByRole('button', { name: 'React' }));
  expect(await screen.findByText('Recent')).toBeInTheDocument();
  expect(screen.getByRole('button', { name: '🎉' })).toBeInTheDocument();
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
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
  });
  await user.click(within(channelDialog).getByRole('button', { name: 'Close dialog' }));
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Private Channels' })
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
    expectActiveTopicBar('kukuri:topic:second');
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
    expectActiveTopicBar('kukuri:topic:second');
  });

  await user.click(screen.getByRole('button', { name: 'Remove kukuri:topic:second' }));

  await waitFor(() => {
    expect(
      screen.queryByRole('button', { name: 'kukuri:topic:second' })
    ).not.toBeInTheDocument();
    expectActiveTopicBar('kukuri:topic:demo');
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

test('desktop shell surfaces relay-assisted topic connectivity in diagnostics', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi({ assistPeerIds: ['relay-peer'] })} />);

  const drawer = await openSettingsSection(user, 'discovery');
  await waitFor(() => {
    expect(within(drawer).getByText('Relay-assisted Peers')).toBeInTheDocument();
    expect(within(drawer).getByText('relay-peer')).toBeInTheDocument();
  });

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:relay');
  await user.click(screen.getByRole('button', { name: 'Add' }));

  await waitFor(() => {
    const relayTopic = screen.getByRole('button', { name: 'kukuri:topic:relay' }).closest('li');
    expect(relayTopic).not.toBeNull();
    expect(relayTopic).toHaveTextContent('relay-assisted / peers: 1');
    expect(relayTopic).not.toHaveTextContent('relay-assisted sync available via 1 peer(s)');
  });

  await user.click(within(drawer).getByTestId('settings-section-connectivity'));
  const relayHeading = await within(drawer).findByRole('heading', { name: 'kukuri:topic:relay' });
  const relaySection = closestSection(relayHeading);
  expect(within(relaySection).getByText('relay-assisted sync available via 1 peer(s)')).toBeInTheDocument();
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
  await screen.findByRole('dialog', { name: 'Settings & diagnostics' });

  fireEvent.keyDown(window, { key: 'Escape' });

  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Settings & diagnostics' })
    ).not.toBeInTheDocument();
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
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'core');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
  });

  await user.click(within(channelDialog).getByRole('button', { name: 'Share' }));

  await waitFor(() => {
    expect(screen.getByText('Share')).toBeInTheDocument();
    expect(screen.getByText(/invite:kukuri:topic:demo:channel-1/i)).toBeInTheDocument();
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
    expectActiveTopicBar('kukuri:topic:private-imported');
    expect(window.location.hash).toBe(
      '#/timeline?topic=kukuri%3Atopic%3Aprivate-imported&channel=channel-imported'
    );
  });
});

test('desktop shell shows friend-only controls and can create a grant', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'friends');
  await user.selectOptions(within(channelDialog).getByLabelText('Audience'), 'friend_only');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(screen.queryByRole('button', { name: 'Rotate' })).not.toBeInTheDocument();
  });

  await user.click(within(channelDialog).getByRole('button', { name: 'Share' }));

  await waitFor(() => {
    expect(screen.getByText('Share')).toBeInTheDocument();
    expect(screen.getByText(/grant:kukuri:topic:demo:channel-1/i)).toBeInTheDocument();
  });
});

test('desktop shell shows friend-plus controls and can create a share', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);
  const channelDialog = await openChannelManager(user);
  await user.type(within(channelDialog).getByPlaceholderText('core contributors'), 'friends+');
  await user.selectOptions(within(channelDialog).getByLabelText('Audience'), 'friend_plus');
  await user.click(within(channelDialog).getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(within(channelDialog).getByRole('button', { name: 'Share' })).toBeInTheDocument();
    expect(window.location.hash).toBe('#/timeline?topic=kukuri%3Atopic%3Ademo&channel=channel-1');
    expect(screen.queryByRole('button', { name: 'Freeze' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Rotate' })).not.toBeInTheDocument();
  });

  await user.click(within(channelDialog).getByRole('button', { name: 'Share' }));

  await waitFor(() => {
    expect(screen.getByText(/share:kukuri:topic:demo:channel-1/i)).toBeInTheDocument();
  });
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
    expect(screen.getByText('syncing image')).toBeInTheDocument();
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
    expect(screen.getByText('syncing image')).toBeInTheDocument();
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
    expect(screen.getByText('image ready')).toBeInTheDocument();
  });
  expect(screen.queryByText('syncing image')).not.toBeInTheDocument();
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
    expect(within(threadPanel).getByText('syncing image')).toBeInTheDocument();
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
    expect(screen.getByText('syncing poster')).toBeInTheDocument();
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
  expect(screen.getAllByText('poster ready').length).toBeGreaterThan(0);
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
  expect(screen.getAllByText('playable video').length).toBeGreaterThan(0);
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
    expect(screen.getAllByText('poster ready').length).toBeGreaterThan(0);
  });

  rerender(<App api={recoveredApi} />);

  const video = await screen.findByTestId('media-video-video-post');
  expect(video).toBeInTheDocument();
  expect(screen.getAllByText('playable video').length).toBeGreaterThan(0);
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
  expect(screen.getAllByText('playable video').length).toBeGreaterThan(0);
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
  expect(screen.getAllByText('unsupported on this client').length).toBeGreaterThan(0);
});

test('community node panel activates relay connectivity on the current session after consent', async () => {
  const api = createDesktopMockApi();
  const user = userEvent.setup();

  render(<App api={api} />);

  const drawer = await openSettingsSection(user, 'community-node');
  await user.type(
    within(drawer).getByPlaceholderText('https://community.example.com'),
    'https://api.kukuri.app'
  );
  await user.click(within(drawer).getByRole('button', { name: 'Save Nodes' }));

  const nodeHeading = await within(drawer).findByRole('heading', { name: 'https://api.kukuri.app' });
  const blockElement = closestSection(nodeHeading);

  await user.click(within(blockElement).getByRole('button', { name: 'Authenticate' }));

  await waitFor(() => {
    expect(
      within(blockElement).getByText('accept required policies to resolve connectivity urls')
    ).toBeInTheDocument();
    expect(within(blockElement).getByText('waiting for consent acceptance')).toBeInTheDocument();
  });

  await user.click(within(blockElement).getByRole('button', { name: 'Accept' }));

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
  await user.type(screen.getByPlaceholderText('Write a message'), 'hello dm');
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
  await user.click(within(conversationIdentity).getByRole('button', { name: 'bob' }));

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
  expect(screen.getAllByRole('button', { name: 'Local Author' }).length).toBeGreaterThan(0);
  expect(screen.getAllByRole('button', { name: 'Bob Display' }).length).toBeGreaterThan(0);
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
  await user.type(screen.getByPlaceholderText('Write a message'), 'hello dm');
  await user.click(screen.getByRole('button', { name: 'Send' }));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
  });

  failNextStatusRefresh = true;
  await new Promise((resolve) => window.setTimeout(resolve, 2300));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
    expect(screen.getByText('temporary dm status failure')).toBeInTheDocument();
    expect(
      screen.queryByText('Direct message send is disabled until the relationship is mutual again.')
    ).not.toBeInTheDocument();
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
    expectActiveTopicBar('kukuri:topic:relay');
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
