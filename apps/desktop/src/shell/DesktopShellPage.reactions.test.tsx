import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { App } from '@/App';
import {
  installObjectUrlMocks,
  openSettingsSection,
  publishPost,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';
import type { DesktopApi } from '@/lib/api';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
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
      'kukuri:topic:general',
      'kukuri:topic:general',
      expect.any(String),
      null
    );
  });
  // The repost renders X-style: the reposter is demoted to a small attribution header.
  expect(await screen.findByText(/reposted$/i)).toBeInTheDocument();
  expect(document.querySelector('.post-repost-attribution')).not.toBeNull();
});

test('desktop shell can create a quote repost from the Column composer', async () => {
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
  await user.click(await screen.findByRole('button', { name: 'Add comment' }));

  const quoteInput = await screen.findByPlaceholderText('Add a comment');
  const composer = quoteInput.closest('form');
  if (!composer) {
    throw new Error('quote repost composer form not found');
  }
  expect(within(composer).getByText('Adding a comment')).toBeInTheDocument();
  expect(within(composer).getByText(/Original post.*source post/)).toBeInTheDocument();
  expect(within(composer).getByLabelText(/attachment/i)).toBeDisabled();

  await user.type(quoteInput, 'quoted take');
  const submitButton = within(composer).getByRole('button', { name: 'Add comment' });
  await user.click(submitButton);

  await waitFor(() => {
    expect(createRepostSpy).toHaveBeenCalledWith(
      'kukuri:topic:general',
      'kukuri:topic:general',
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
      'kukuri:topic:general': [
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
          published_topic_id: 'kukuri:topic:general',
          origin_topic_id: 'kukuri:topic:general',
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

  const remoteReactionImage = await screen.findByAltText(remoteReactionAsset.search_key);
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

