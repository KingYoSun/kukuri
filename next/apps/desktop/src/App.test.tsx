import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';

import { App } from './App';
import { DesktopApi, SyncStatus, TimelineView } from './lib/api';

function createMockApi() {
  let posts: TimelineView['items'] = [];
  let sequence = 0;
  const syncStatus: SyncStatus = {
    connected: true,
    last_sync_ts: 1,
    peer_count: 1,
    pending_events: 0,
    subscribed_topics: ['kukuri:topic:demo'],
  };

  const api: DesktopApi = {
    async createPost(topic, content, replyTo) {
      sequence += 1;
      const id = `${topic}-${sequence}`;
      const rootId = replyTo
        ? posts.find((post) => post.id === replyTo)?.root_id ?? replyTo
        : id;
      posts = [
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
      return id;
    },
    async listTimeline() {
      return { items: posts, next_cursor: null };
    },
    async listThread(_topic, threadId) {
      return { items: posts.filter((post) => post.root_id === threadId || post.id === threadId), next_cursor: null };
    },
    async getSyncStatus() {
      return syncStatus;
    },
    async importPeerTicket() {},
    async getLocalPeerTicket() {
      return 'peer1@127.0.0.1:7777';
    },
  };

  return api;
}

test('desktop shell can publish and render a post', async () => {
  const user = userEvent.setup();
  render(<App api={createMockApi()} />);

  await user.clear(screen.getByLabelText('Topic'));
  await user.type(screen.getByLabelText('Topic'), 'kukuri:topic:demo');
  await user.type(screen.getByPlaceholderText('Write a post'), 'hello desktop');
  await user.click(screen.getByRole('button', { name: 'Publish' }));

  await waitFor(() => {
    expect(screen.getByText('hello desktop')).toBeInTheDocument();
  });
  expect(screen.getByText('Live over static peers')).toBeInTheDocument();
  expect(screen.getByDisplayValue('peer1@127.0.0.1:7777')).toBeInTheDocument();
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
