import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test } from 'vitest';

import { App } from './App';
import { DesktopApi, SyncStatus, TimelineView } from './lib/api';

function createMockApi() {
  let posts: TimelineView['items'] = [];
  const syncStatus: SyncStatus = {
    connected: true,
    last_sync_ts: 1,
    peer_count: 1,
    pending_events: 0,
    subscribed_topics: ['kukuri:topic:demo'],
  };

  const api: DesktopApi = {
    async createPost(topic, content) {
      const id = `${topic}-${posts.length + 1}`;
      posts = [
        {
          id,
          author_pubkey: 'f'.repeat(64),
          author_npub: 'npub1testauthor',
          note_id: `note1${posts.length + 1}`,
          content,
          created_at: 1,
          reply_to: null,
          root_id: id,
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
