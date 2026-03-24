import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { App } from './App';
import {
  AttachmentView,
  BlobViewStatus,
  CreateAttachmentInput,
  PostView,
} from './lib/api';

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
  await user.click(screen.getByTestId('shell-settings-trigger'));
  return await screen.findByRole('dialog', { name: 'Settings & diagnostics' });
}

async function openSettingsSection(
  user: ReturnType<typeof userEvent.setup>,
  section: 'profile' | 'connectivity' | 'discovery' | 'community-node'
) {
  const drawer = await openSettingsDrawer(user);
  if (section !== 'profile') {
    await user.click(within(drawer).getByTestId(`settings-section-${section}`));
  }
  return drawer;
}

test('desktop shell can publish and render a post', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('Write a post'), 'hello desktop');
  await user.click(screen.getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(screen.getByText('hello desktop')).toBeInTheDocument();
  });
  expect(screen.getByText('Seeded DHT + direct peers')).toBeInTheDocument();
  expect(screen.getByText('joined / peers: 1')).toBeInTheDocument();

  const drawer = await openSettingsSection(user, 'connectivity');
  expect(within(drawer).getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  expect(within(drawer).getByText('Configured Peers')).toBeInTheDocument();
  expect(within(drawer).getByText('Connected to all configured peers')).toBeInTheDocument();
  expect(within(drawer).getAllByText('peer-a').length).toBeGreaterThan(0);
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

  await user.type(screen.getAllByPlaceholderText('Write a post')[0], 'root post');
  await user.click(screen.getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Reply' }));
  expect(screen.getByText('Replying')).toBeInTheDocument();
  expect(screen.getByPlaceholderText('Write a reply')).toBeInTheDocument();

  const replyInput = screen.getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  const submitButton = composer.querySelector('button[type="submit"]');
  if (!(submitButton instanceof HTMLButtonElement)) {
    throw new Error('reply submit button not found');
  }
  await user.click(submitButton);

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(screen.getAllByText('Reply').length).toBeGreaterThan(0);
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

  await user.type(screen.getAllByPlaceholderText('Write a post')[0], 'root post');
  await user.click(screen.getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.getByText('root post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Reply' }));
  await waitFor(() => {
    expect(screen.getByPlaceholderText('Write a reply')).toBeInTheDocument();
  });
  const threadCallsBeforeSubmit = listThreadSpy.mock.calls.length;

  const replyInput = screen.getByPlaceholderText('Write a reply');
  await user.type(replyInput, 'reply post');
  const composer = replyInput.closest('form');
  if (!composer) {
    throw new Error('reply composer form not found');
  }
  const submitButton = composer.querySelector('button[type="submit"]');
  if (!(submitButton instanceof HTMLButtonElement)) {
    throw new Error('reply submit button not found');
  }
  await user.click(submitButton);

  await waitFor(() => {
    expect(screen.getAllByText('reply post').length).toBeGreaterThan(0);
  });
  expect(listThreadSpy.mock.calls.length - threadCallsBeforeSubmit).toBe(1);
});

test('desktop shell can track multiple topics at once', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  expect(screen.getByRole('button', { name: 'kukuri:topic:second' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  await user.type(screen.getByPlaceholderText('Write a post'), 'demo post');
  await user.click(screen.getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.getByText('demo post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));
  await user.type(screen.getByPlaceholderText('Write a post'), 'second post');
  await user.click(screen.getByRole('button', { name: 'Publish' }));
  await waitFor(() => {
    expect(screen.getByText('second post')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'kukuri:topic:demo' }));
  expect(screen.getByText('demo post')).toBeInTheDocument();
  expect(screen.getByText('idle / peers: 0')).toBeInTheDocument();
  expect(screen.getByText('expected: 0')).toBeInTheDocument();
  expect(
    screen.getByText('Connected to all configured peers for this topic')
  ).toBeInTheDocument();
});

test('removing the active topic falls back to the remaining tracked topic', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('kukuri:topic:demo'), 'kukuri:topic:second');
  await user.click(screen.getByRole('button', { name: 'Add' }));
  await user.click(screen.getByRole('button', { name: 'kukuri:topic:second' }));

  await waitFor(() => {
    expect(screen.getByText('Active topic: kukuri:topic:second')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Remove kukuri:topic:second' }));

  await waitFor(() => {
    expect(
      screen.queryByRole('button', { name: 'kukuri:topic:second' })
    ).not.toBeInTheDocument();
    expect(screen.getByText('Active topic: kukuri:topic:demo')).toBeInTheDocument();
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
    expect(screen.getByRole('tab', { name: 'Thread' })).toHaveAttribute('aria-selected', 'true');
  });
  expect(screen.getByRole('tabpanel', { name: 'Thread' })).toBeInTheDocument();

  await user.click(screen.getAllByRole('button', { name: 'bob' })[0]);

  await waitFor(() => {
    expect(screen.getByRole('tab', { name: 'Author' })).toHaveAttribute('aria-selected', 'true');
  });
  expect(screen.getByText('Author Detail')).toBeInTheDocument();
  expect(screen.getByText('author detail from timeline')).toBeInTheDocument();
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
    expect(relayTopic).toHaveTextContent('relay-assisted sync available via 1 peer(s)');
  });
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
    expect(within(drawer).getByText('Last Error')).toBeInTheDocument();
    expect(
      within(drawer).getByText('failed to import peer ticket: invalid endpoint id')
    ).toBeInTheDocument();
    expect(
      screen.getByText('error: timed out waiting for gossip topic join')
    ).toBeInTheDocument();
  });
});

test('desktop shell primary nav jumps focus and settings drawer restores trigger focus on escape', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  const gameNav = screen.getByRole('button', { name: /Game/i });
  await user.click(gameNav);

  const gameSection = screen.getByPlaceholderText('Top 8 Finals').closest('.shell-section');
  if (!(gameSection instanceof HTMLElement)) {
    throw new Error('game section not found');
  }

  await waitFor(() => {
    expect(gameNav).toHaveAttribute('aria-current', 'location');
    expect(gameSection).toHaveFocus();
  });

  const settingsTrigger = screen.getByTestId('shell-settings-trigger');
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

  await user.type(screen.getByPlaceholderText('Friday stream'), 'Launch Party');
  await user.type(screen.getByPlaceholderText('short session summary'), 'watch along');
  await user.click(screen.getByRole('button', { name: 'Start Live' }));

  await waitFor(() => {
    expect(screen.getByText('Launch Party')).toBeInTheDocument();
  });
  expect(screen.getByText('watch along')).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Join' }));
  await waitFor(() => {
    expect(screen.getByText('viewers: 1')).toBeInTheDocument();
  });
  expect(screen.getByRole('button', { name: 'Leave' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Leave' }));
  await waitFor(() => {
    expect(screen.getByText('viewers: 0')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'End' }));
  await waitFor(() => {
    expect(screen.getByText('Ended')).toBeInTheDocument();
  });
});

test('desktop shell can create a private channel and export an invite', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  expect(screen.queryByRole('button', { name: 'Create Invite' })).not.toBeInTheDocument();

  await user.type(screen.getByPlaceholderText('core contributors'), 'core');
  await user.click(screen.getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(screen.getByLabelText('Compose Target')).toHaveValue('channel:channel-1');
    expect(screen.getByText(/Posting to: core/i)).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Create Invite' }));

  await waitFor(() => {
    expect(screen.getByText('Latest invite')).toBeInTheDocument();
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
          expires_at: null,
          namespace_secret_hex: 'a'.repeat(64),
        },
      })}
    />
  );

  await user.type(
    screen.getByPlaceholderText(/paste private channel invite/i),
    'invite-token'
  );
  await user.click(screen.getByRole('button', { name: 'Join Invite' }));

  await waitFor(() => {
    expect(
      screen.getByRole('button', { name: 'kukuri:topic:private-imported' })
    ).toBeInTheDocument();
    expect(screen.getByLabelText('View Scope')).toHaveValue('channel:channel-imported');
    expect(screen.getByLabelText('Compose Target')).toHaveValue('channel:channel-imported');
    expect(screen.getByText(/Viewing: Imported/i)).toBeInTheDocument();
    expect(screen.getByText(/Posting to: Imported/i)).toBeInTheDocument();
  });
});

test('desktop shell shows friend-only controls and can create a grant', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('core contributors'), 'friends');
  await user.selectOptions(screen.getByLabelText('Channel Audience'), 'friend_only');
  await user.click(screen.getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(screen.getByText(/Policy: Friends: only mutual followers can join/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Grant' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Rotate' })).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Create Grant' }));

  await waitFor(() => {
    expect(screen.getByText('Latest grant')).toBeInTheDocument();
    expect(screen.getByText(/grant:kukuri:topic:demo:channel-1/i)).toBeInTheDocument();
  });
});

test('desktop shell shows friend-plus controls and can create a share', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('core contributors'), 'friends+');
  await user.selectOptions(screen.getByLabelText('Channel Audience'), 'friend_plus');
  await user.click(screen.getByRole('button', { name: 'Create Channel' }));

  await waitFor(() => {
    expect(
      screen.getByText(/Policy: Friends\+: participants can share to their mutuals/i)
    ).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Create Share' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Freeze' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Rotate' })).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Create Share' }));

  await waitFor(() => {
    expect(screen.getByText('Latest share')).toBeInTheDocument();
    expect(screen.getByText(/share:kukuri:topic:demo:channel-1/i)).toBeInTheDocument();
  });
});

test('desktop shell can create and update a game room', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.type(screen.getByPlaceholderText('Top 8 Finals'), 'Grand Finals');
  await user.type(screen.getByPlaceholderText('match summary'), 'set one');
  await user.type(screen.getByPlaceholderText('Alice, Bob'), 'Alice, Bob');
  await user.click(screen.getByRole('button', { name: 'Create Room' }));

  await waitFor(() => {
    expect(screen.getByText('Grand Finals')).toBeInTheDocument();
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

  await user.upload(screen.getByLabelText('Attach'), [
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' }),
    new File([Uint8Array.from([5, 6, 7, 8])], 'clip.mp4', { type: 'video/mp4' }),
  ]);
  await waitFor(() => {
    expect(screen.getByText('flower.png')).toBeInTheDocument();
    expect(screen.getByText('clip.mp4')).toBeInTheDocument();
  });
  await user.click(screen.getByRole('button', { name: 'Publish' }));

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

  await user.upload(
    screen.getByLabelText('Attach'),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  });
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Publish' }));

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

  await user.upload(
    screen.getByLabelText('Attach'),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );
  await user.click(screen.getByRole('button', { name: 'Publish' }));

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

  await user.upload(
    screen.getByLabelText('Attach'),
    new File([Uint8Array.from([1, 3, 5, 7])], 'broken.mp4', { type: 'video/mp4' })
  );

  await waitFor(() => {
    expect(screen.getByText('failed to generate video poster')).toBeInTheDocument();
  });

  await user.click(screen.getByRole('button', { name: 'Publish' }));

  expect(createPostSpy).not.toHaveBeenCalled();
});

test('composer shows image draft preview before publish', async () => {
  installObjectUrlMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.upload(
    screen.getByLabelText('Attach'),
    new File([Uint8Array.from([1, 2, 3, 4])], 'flower.png', { type: 'image/png' })
  );

  expect(await screen.findByRole('img', { name: 'draft preview flower.png' })).toBeInTheDocument();
  expect(screen.getByText(/image_original/)).toBeInTheDocument();
  expect(screen.getByText(/image\/png/)).toBeInTheDocument();
  expect(screen.getByText(/4 B/)).toBeInTheDocument();
});

test('composer shows video poster draft preview before publish', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi()} />);

  await user.upload(
    screen.getByLabelText('Attach'),
    new File([Uint8Array.from([7, 8, 9])], 'clip.mp4', { type: 'video/mp4' })
  );

  expect(await screen.findByRole('img', { name: 'draft preview clip.mp4' })).toBeInTheDocument();
  expect(screen.getByText(/video_manifest/)).toBeInTheDocument();
  expect(screen.getByText(/video_poster/)).toBeInTheDocument();
  expect(screen.getByText(/image\/jpeg/)).toBeInTheDocument();
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
  const threadPanel = await screen.findByRole('tabpanel', { name: 'Thread' });

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

  const nodeHeading = await within(drawer).findByText(
    (_content, element) =>
      element?.tagName === 'STRONG' && element.textContent === 'https://api.kukuri.app'
  );
  const block = nodeHeading.closest('.diagnostic-block') as HTMLElement | null;
  expect(block).not.toBeNull();
  const blockElement = block as HTMLElement;

  await user.click(within(blockElement).getByRole('button', { name: 'Authenticate' }));

  await waitFor(() => {
    expect(within(blockElement).getByText(/next step:/i)).toHaveTextContent(
      'accept required policies to resolve connectivity urls'
    );
    expect(within(blockElement).getByText(/session activation:/i)).toHaveTextContent(
      'waiting for consent acceptance'
    );
  });

  await user.click(within(blockElement).getByRole('button', { name: 'Accept' }));

  await waitFor(() => {
    expect(within(blockElement).getByText(/connectivity urls:/i)).toHaveTextContent(
      'https://api.kukuri.app'
    );
    expect(within(blockElement).getByText(/session activation:/i)).toHaveTextContent(
      'active on current session'
    );
    expect(within(blockElement).getByText(/next step:/i)).toHaveTextContent(
      'connectivity urls active on current session'
    );
  });
});

test('context pane auto-switches between author and thread and keeps manual tab selection', async () => {
  const user = userEvent.setup();
  const authorPubkey = 'a'.repeat(64);

  render(
    <App
      api={createDesktopMockApi({
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
      })}
    />
  );

  await user.click(await screen.findByRole('button', { name: 'alice' }));
  await waitFor(() => {
    expect(screen.getByRole('tab', { name: 'Author' })).toHaveAttribute('aria-selected', 'true');
  });

  await user.click(screen.getByText('envelope-context-post'));
  await waitFor(() => {
    expect(screen.getByRole('tab', { name: 'Thread' })).toHaveAttribute('aria-selected', 'true');
  });

  await user.click(screen.getByRole('tab', { name: 'Author' }));
  expect(screen.getByRole('tab', { name: 'Author' })).toHaveAttribute('aria-selected', 'true');

  await user.click(screen.getByRole('button', { name: 'Clear Author' }));
  const authorPanel = screen.getByRole('tabpanel', { name: 'Author' });
  expect(screen.getByRole('tab', { name: 'Author' })).toHaveAttribute('aria-selected', 'true');
  expect(
    within(authorPanel).getByText('Select an author to inspect profile and relationship.')
  ).toBeInTheDocument();
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

  expect(await screen.findByText('Author Detail')).toBeInTheDocument();
  expect(screen.getByText(`${viaA.slice(0, 12)}, ${viaB.slice(0, 12)}`)).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Follow' })).toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Follow' }));

  await waitFor(() => {
    expect(screen.getByRole('button', { name: 'Unfollow' })).toBeInTheDocument();
  });
  expect(screen.getAllByText('following').length).toBeGreaterThan(0);
});

test('local profile editor saves profile draft', async () => {
  const api = createDesktopMockApi();
  const user = userEvent.setup();

  render(<App api={api} />);

  const drawer = await openSettingsSection(user, 'profile');
  const displayNameInput = within(drawer).getByPlaceholderText('Visible label');
  await user.type(displayNameInput, 'Local Author');
  await user.click(within(drawer).getByRole('button', { name: 'Save Profile' }));

  await waitFor(() => {
    expect(screen.getByText('Local Author')).toBeInTheDocument();
  });
});

test('keeps local peer ticket visible when profile loading fails', async () => {
  const user = userEvent.setup();
  render(<App api={createDesktopMockApi({ myProfileError: 'profile load failed' })} />);

  const drawer = await openSettingsSection(user, 'connectivity');
  await waitFor(() => {
    expect(within(drawer).getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  });
});
