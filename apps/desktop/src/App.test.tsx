import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';

import { App } from './App';
import { AttachmentView, BlobViewStatus, DesktopApi, PostView, SyncStatus, TimelineView } from './lib/api';

function createMockApi(options?: {
  globalLastError?: string | null;
  topicLastError?: string | null;
  seedPosts?: Record<string, TimelineView['items']>;
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
  };

  const api: DesktopApi = {
    async createPost(topic, content, replyTo) {
      sequence += 1;
      const id = `${topic}-${sequence}`;
      const posts = postsByTopic[topic] ?? [];
      const rootId = replyTo
        ? posts.find((post) => post.id === replyTo)?.root_id ?? replyTo
        : id;
      postsByTopic[topic] = [
        {
          id,
          author_pubkey: 'f'.repeat(64),
          author_npub: 'npub1testauthor',
          note_id: `note1${sequence}`,
          content,
          content_status: 'Available',
          attachments: [],
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
    async getSyncStatus() {
      return syncStatus;
    },
    async importPeerTicket() {},
    async unsubscribeTopic(topic) {
      delete postsByTopic[topic];
      syncStatus.subscribed_topics = syncStatus.subscribed_topics.filter((value) => value !== topic);
      syncStatus.topic_diagnostics = syncStatus.topic_diagnostics.filter((value) => value.topic !== topic);
    },
    async getLocalPeerTicket() {
      return 'peer1@127.0.0.1:7777';
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

test('timeline image post shows media skeleton when attachment is missing', async () => {
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

test('timeline image post renders actual preview when preview url is available', async () => {
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
  api.getBlobPreviewUrl = async () => 'data:image/png;base64,ZmFrZQ==';

  render(<App api={api} />);

  await waitFor(() => {
    expect(screen.getByTestId('media-preview-image-post')).toBeInTheDocument();
  });
});

test('thread pane reuses the same image placeholder renderer', async () => {
  const user = userEvent.setup();
  render(
    <App
      api={createMockApi({
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
      })}
    />
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
