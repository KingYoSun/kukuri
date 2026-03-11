import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';

import { App } from './App';
import { DesktopApi, SyncStatus, TimelineView } from './lib/api';

function createMockApi() {
  const postsByTopic: Record<string, TimelineView['items']> = {};
  let sequence = 0;
  const syncStatus: SyncStatus = {
    connected: true,
    last_sync_ts: 1,
    peer_count: 1,
    pending_events: 0,
    subscribed_topics: ['kukuri:topic:demo'],
    topic_diagnostics: [
      {
        topic: 'kukuri:topic:demo',
        joined: true,
        peer_count: 1,
        connected_peers: ['peer-a'],
        last_received_at: 1,
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
          last_received_at: sequence,
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
          last_received_at: null,
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
  };

  return api;
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
});
