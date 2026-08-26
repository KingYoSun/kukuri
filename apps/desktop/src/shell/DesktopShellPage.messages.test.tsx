import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  getDetailPane,
  getActiveColumn,
  installObjectUrlMocks,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { DesktopApi, DirectMessageMessageView } from '@/lib/api';
import { REFRESH_INTERVAL_MS } from '@/shell/store';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

async function expandConversationComposer(user: ReturnType<typeof userEvent.setup>) {
  const conversationColumn = await screen.findByRole('region', { name: /Conversation Column/ });
  await user.click(
    within(conversationColumn).getByRole('button', { name: /Message to / })
  );
  return conversationColumn;
}

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
    expect(getActiveColumn('Conversation')).toHaveAttribute(
      'aria-current',
      'true'
    );
    expect(window.location.hash).toBe(
      `#/messages?topic=kukuri%3Atopic%3Ademo&peerPubkey=${authorPubkey}`
    );
  });

  const conversationColumn = await expandConversationComposer(user);

  await waitFor(() => {
    expect(within(conversationColumn).getByPlaceholderText('Write a message')).not.toBeDisabled();
    expect(within(conversationColumn).getByRole('button', { name: 'Send' })).not.toBeDisabled();
  });
  fireEvent.change(within(conversationColumn).getByPlaceholderText('Write a message'), {
    target: { value: 'hello dm' },
  });
  await user.click(within(conversationColumn).getByRole('button', { name: 'Send' }));

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
    expect(screen.getByRole('region', { name: /^Conversation Column,/ })).toBeInTheDocument();
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
  const user = userEvent.setup();

  await waitFor(() => {
    expect(screen.getByRole('region', { name: /^Conversation Column,/ })).toBeInTheDocument();
    expect(getDetailPane('Author')).toBeInTheDocument();
  });
  const conversationColumn = await expandConversationComposer(user);
  expect(within(conversationColumn).getByPlaceholderText('Write a message')).toBeInTheDocument();
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
    expect(getActiveColumn('Conversation')).toHaveAttribute(
      'aria-current',
      'true'
    );
  });
  const conversationColumn = await expandConversationComposer(user);
  await waitFor(() => {
    expect(within(conversationColumn).getByPlaceholderText('Write a message')).not.toBeDisabled();
    expect(within(conversationColumn).getByRole('button', { name: 'Send' })).not.toBeDisabled();
  });
  fireEvent.change(within(conversationColumn).getByPlaceholderText('Write a message'), {
    target: { value: 'hello dm' },
  });
  await user.click(within(conversationColumn).getByRole('button', { name: 'Send' }));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
  });

  failNextStatusRefresh = true;
  await new Promise((resolve) => window.setTimeout(resolve, REFRESH_INTERVAL_MS + 300));

  await waitFor(() => {
    expect(screen.getAllByText('hello dm').length).toBeGreaterThan(0);
    expect(screen.getAllByText('temporary dm status failure').length).toBeGreaterThan(0);
    expect(
      screen.queryByText('Direct message send is disabled until the relationship is mutual again.')
    ).not.toBeInTheDocument();
  });
});

