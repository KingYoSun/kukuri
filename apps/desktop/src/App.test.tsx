import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { App } from './App';
import {
  AttachmentView,
  BlobViewStatus,
  CreateAttachmentInput,
  DesktopApi,
  GameRoomView,
  GameScoreView,
  LiveSessionView,
  PostView,
  SyncStatus,
  TimelineView,
} from './lib/api';

function createMockApi(options?: {
  globalLastError?: string | null;
  topicLastError?: string | null;
  seedPosts?: Record<string, TimelineView['items']>;
  seedLiveSessions?: Record<string, LiveSessionView[]>;
  seedGameRooms?: Record<string, GameRoomView[]>;
}) {
  const postsByTopic: Record<string, TimelineView['items']> = Object.fromEntries(
    Object.entries(options?.seedPosts ?? {}).map(([topic, posts]) => [
      topic,
      posts.map((post) => ({
        ...post,
        attachments: [...post.attachments],
      })),
    ])
  );
  const liveSessionsByTopic: Record<string, LiveSessionView[]> = Object.fromEntries(
    Object.entries(options?.seedLiveSessions ?? {}).map(([topic, sessions]) => [topic, [...sessions]])
  );
  const gameRoomsByTopic: Record<string, GameRoomView[]> = Object.fromEntries(
    Object.entries(options?.seedGameRooms ?? {}).map(([topic, rooms]) => [
      topic,
      rooms.map((room) => ({
        ...room,
        scores: room.scores.map((score) => ({ ...score })),
      })),
    ])
  );
  let sequence = 0;
  const syncStatus: SyncStatus = {
    connected: true,
    last_sync_ts: 1,
    peer_count: 1,
    pending_events: 0,
    status_detail: 'Connected to all configured peers',
    last_error: options?.globalLastError ?? null,
    configured_peers: ['peer-a'],
    subscribed_topics: ['kukuri:topic:demo'],
    topic_diagnostics: [
      {
        topic: 'kukuri:topic:demo',
        joined: true,
        peer_count: 1,
        connected_peers: ['peer-a'],
        configured_peer_ids: ['peer-a'],
        missing_peer_ids: [],
        last_received_at: 1,
        status_detail: 'Connected to all configured peers for this topic',
        last_error: options?.topicLastError ?? null,
      },
    ],
    local_author_pubkey: 'f'.repeat(64),
  };

  const api: DesktopApi = {
    async createPost(topic, content, replyTo, attachments) {
      sequence += 1;
      const id = `${topic}-${sequence}`;
      const posts = postsByTopic[topic] ?? [];
      const rootId = replyTo
        ? posts.find((post) => post.id === replyTo)?.root_id ?? replyTo
        : id;
      const postAttachments: AttachmentView[] = (attachments ?? []).map((attachment, index) => ({
        hash: `${id}-attachment-${index}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }));
      postsByTopic[topic] = [
        {
          id,
          author_pubkey: 'f'.repeat(64),
          author_npub: 'npub1testauthor',
          note_id: `note1${sequence}`,
          content,
          content_status: 'Available',
          attachments: postAttachments,
          created_at: sequence,
          reply_to: replyTo ?? null,
          root_id: rootId,
        },
        ...posts,
      ];
      syncStatus.subscribed_topics = Array.from(new Set([...syncStatus.subscribed_topics, topic]));
      if (!syncStatus.topic_diagnostics.some((entry) => entry.topic === topic)) {
        syncStatus.topic_diagnostics.push({
          topic,
          joined: true,
          peer_count: 1,
          connected_peers: ['peer-a'],
          configured_peer_ids: ['peer-a'],
          missing_peer_ids: [],
          last_received_at: sequence,
          status_detail: 'Connected to all configured peers for this topic',
          last_error: null,
        });
      }
      return id;
    },
    async listTimeline(topic) {
      syncStatus.subscribed_topics = Array.from(new Set([...syncStatus.subscribed_topics, topic]));
      if (!syncStatus.topic_diagnostics.some((entry) => entry.topic === topic)) {
        syncStatus.topic_diagnostics.push({
          topic,
          joined: false,
          peer_count: 0,
          connected_peers: [],
          configured_peer_ids: [],
          missing_peer_ids: [],
          last_received_at: null,
          status_detail: 'No peers configured for this topic',
          last_error: null,
        });
      }
      return { items: postsByTopic[topic] ?? [], next_cursor: null };
    },
    async listThread(topic, threadId) {
      const posts = postsByTopic[topic] ?? [];
      return {
        items: posts.filter((post) => post.root_id === threadId || post.id === threadId),
        next_cursor: null,
      };
    },
    async listLiveSessions(topic) {
      return liveSessionsByTopic[topic] ?? [];
    },
    async createLiveSession(topic, title, description) {
      sequence += 1;
      const sessionId = `live-${sequence}`;
      liveSessionsByTopic[topic] = [
        {
          session_id: sessionId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Live',
          started_at: Date.now(),
          ended_at: null,
          viewer_count: 0,
          joined_by_me: false,
        },
        ...(liveSessionsByTopic[topic] ?? []),
      ];
      return sessionId;
    },
    async endLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? {
              ...session,
              status: 'Ended',
              ended_at: Date.now(),
              joined_by_me: false,
            }
          : session
      );
    },
    async joinLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? {
              ...session,
              joined_by_me: true,
              viewer_count: session.viewer_count + 1,
            }
          : session
      );
    },
    async leaveLiveSession(topic, sessionId) {
      liveSessionsByTopic[topic] = (liveSessionsByTopic[topic] ?? []).map((session) =>
        session.session_id === sessionId
          ? {
              ...session,
              joined_by_me: false,
              viewer_count: Math.max(0, session.viewer_count - 1),
            }
          : session
      );
    },
    async listGameRooms(topic) {
      return gameRoomsByTopic[topic] ?? [];
    },
    async createGameRoom(topic, title, description, participants) {
      sequence += 1;
      const roomId = `game-${sequence}`;
      const scores: GameScoreView[] = participants.map((label, index) => ({
        participant_id: `participant-${index + 1}`,
        label,
        score: 0,
      }));
      gameRoomsByTopic[topic] = [
        {
          room_id: roomId,
          host_pubkey: syncStatus.local_author_pubkey,
          title,
          description,
          status: 'Open',
          phase_label: null,
          scores,
          updated_at: Date.now(),
        },
        ...(gameRoomsByTopic[topic] ?? []),
      ];
      return roomId;
    },
    async updateGameRoom(topic, roomId, status, phaseLabel, scores) {
      gameRoomsByTopic[topic] = (gameRoomsByTopic[topic] ?? []).map((room) =>
        room.room_id === roomId
          ? {
              ...room,
              status,
              phase_label: phaseLabel,
              scores: scores.map((score) => ({ ...score })),
              updated_at: Date.now(),
            }
          : room
      );
    },
    async getSyncStatus() {
      return syncStatus;
    },
    async importPeerTicket() {},
    async unsubscribeTopic(topic) {
      delete postsByTopic[topic];
      delete liveSessionsByTopic[topic];
      delete gameRoomsByTopic[topic];
      syncStatus.subscribed_topics = syncStatus.subscribed_topics.filter((value) => value !== topic);
      syncStatus.topic_diagnostics = syncStatus.topic_diagnostics.filter((value) => value.topic !== topic);
    },
    async getLocalPeerTicket() {
      return 'peer1@127.0.0.1:7777';
    },
    async getBlobMediaPayload(_hash, mime) {
      return {
        bytes_base64: mime.startsWith('video/') ? 'ZmFrZS12aWRlbw==' : 'ZmFrZS1pbWFnZQ==',
        mime,
      };
    },
    async getBlobPreviewUrl() {
      return null;
    },
  };

  return api;
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
    id: 'image-post',
    author_pubkey: 'f'.repeat(64),
    author_npub: 'npub1imageauthor',
    note_id: 'note1imagepost',
    content: '[blob pending]',
    content_status: 'Missing',
    attachments: [attachment],
    created_at: 1,
    reply_to: null,
    root_id: 'image-post',
    ...overrides,
  };
}

function buildVideoPost(overrides?: Partial<PostView>): PostView {
  return {
    id: 'video-post',
    author_pubkey: 'e'.repeat(64),
    author_npub: 'npub1videoauthor',
    note_id: 'note1videopost',
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

test('desktop shell can publish and render a post', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

  await user.type(screen.getByPlaceholderText('Write a post'), 'hello desktop');
  await user.click(screen.getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(screen.getByText('hello desktop')).toBeInTheDocument();
  });
  expect(screen.getByText('Live over static peers')).toBeInTheDocument();
  expect(screen.getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
  expect(screen.getByText('Configured Peers')).toBeInTheDocument();
  expect(screen.getByText('Connected to all configured peers')).toBeInTheDocument();
  expect(screen.getAllByText('peer-a').length).toBeGreaterThan(0);
  expect(screen.getByText('joined / peers: 1')).toBeInTheDocument();
});

test('desktop shell can enter reply mode and render reply state', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

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

test('desktop shell can track multiple topics at once', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

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

test('desktop shell renders diagnostics error reasons', async () => {
  render(
    <App
      api={createMockApi({
        globalLastError: 'failed to import peer ticket: invalid endpoint id',
        topicLastError: 'timed out waiting for gossip topic join',
      })}
    />
  );

  await waitFor(() => {
    expect(screen.getByText('Last Error')).toBeInTheDocument();
    expect(
      screen.getByText('failed to import peer ticket: invalid endpoint id')
    ).toBeInTheDocument();
    expect(
      screen.getByText('error: timed out waiting for gossip topic join')
    ).toBeInTheDocument();
  });
});

test('desktop shell can create, join, leave, and end a live session', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

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

test('desktop shell can create and update a game room', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

  await user.type(screen.getByPlaceholderText('Top 8 Finals'), 'Grand Finals');
  await user.type(screen.getByPlaceholderText('match summary'), 'set one');
  await user.type(screen.getByPlaceholderText('Alice, Bob'), 'Alice, Bob');
  await user.click(screen.getByRole('button', { name: 'Create Room' }));

  await waitFor(() => {
    expect(screen.getByText('Grand Finals')).toBeInTheDocument();
  });
  expect(screen.getByText('set one')).toBeInTheDocument();
  expect(screen.getByLabelText(/game-.*-Alice-score/)).toBeInTheDocument();

  await user.selectOptions(screen.getByLabelText(/game-.*-status/), 'InProgress');
  await user.clear(screen.getByLabelText(/game-.*-phase/));
  await user.type(screen.getByLabelText(/game-.*-phase/), 'Round 3');
  await user.clear(screen.getByLabelText(/game-.*-Alice-score/));
  await user.type(screen.getByLabelText(/game-.*-Alice-score/), '2');
  await user.click(screen.getByRole('button', { name: 'Save Room' }));

  await waitFor(() => {
    expect(screen.getByLabelText(/game-.*-status/)).toHaveValue('InProgress');
  });
  expect(screen.getByText('phase: Round 3')).toBeInTheDocument();
  expect(screen.getByDisplayValue('2')).toBeInTheDocument();
});

test('single attach button classifies mixed image and video files', async () => {
  installObjectUrlMocks();
  installSuccessfulPosterGenerationMocks();
  let attachmentsSeen: CreateAttachmentInput[] = [];
  const api = createMockApi();
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
  const api = createMockApi();
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
  const api = createMockApi();
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
  const api = createMockApi();
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
  render(<App api={createMockApi()} />);

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
  render(<App api={createMockApi()} />);

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
  const api = createMockApi({
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
      api={createMockApi({
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
      api={createMockApi({
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
  const api = createMockApi({
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
  const api = createMockApi({
    seedPosts: {
      'kukuri:topic:demo': [
        buildImagePost(),
        {
          ...buildImagePost({
            id: 'reply-post',
            note_id: 'note1replypost',
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
    expect(screen.getByText('note1imagepost')).toBeInTheDocument();
  });
  await user.click(screen.getByText('note1imagepost'));
  const threadPanel = screen.getByRole('heading', { name: 'Thread' }).closest('section');
  if (!threadPanel) {
    throw new Error('thread panel not found');
  }

  await waitFor(() => {
    expect(within(threadPanel).getByText('syncing image')).toBeInTheDocument();
  });
  expect(within(threadPanel).getByTestId('media-skeleton-image-post')).toBeInTheDocument();
});

test('text body pending uses text skeleton without hiding image metadata', async () => {
  render(
    <App
      api={createMockApi({
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
  const api = createMockApi({
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
  const api = createMockApi({
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
  const api = createMockApi({
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
  const stalledApi = createMockApi({
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
  const recoveredApi = createMockApi({
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
  const api = createMockApi({
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
  const api = createMockApi({
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
