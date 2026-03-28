import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { PostCard } from './PostCard';
import { type PostCardView } from './types';

function createView(overrides?: Partial<PostCardView>): PostCardView {
  return {
    post: {
      object_id: 'post-1',
      envelope_id: 'envelope-post-1',
      author_pubkey: 'a'.repeat(64),
      author_name: 'alice',
      author_display_name: 'Alice',
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: false,
      object_kind: 'post',
      content: 'hello',
      content_status: 'Available',
      attachments: [],
      created_at: 1,
      reply_to: null,
      root_id: 'post-1',
      published_topic_id: null,
      origin_topic_id: null,
      repost_of: null,
      repost_commentary: null,
      is_threadable: true,
      channel_id: null,
      audience_label: 'Public',
      reaction_summary: [],
      my_reactions: [],
    },
    context: 'timeline',
    authorLabel: 'Alice',
    authorPicture: null,
    relationshipLabel: null,
    audienceChipLabel: 'core contributors',
    threadTargetId: 'post-1',
    media: {
      objectId: 'post-1',
      kind: null,
      statusLabel: null,
      extraAttachmentCount: 0,
      state: 'ready',
      metaMime: null,
      metaBytesLabel: null,
      imagePreviewSrc: null,
      videoPosterPreviewSrc: null,
      videoPlaybackSrc: null,
      videoUnsupportedOnClient: false,
    },
    ...overrides,
  };
}

test('post card hides the object kind and shows a placeholder avatar when no picture is available', () => {
  render(
    <PostCard
      view={createView()}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  expect(screen.queryByText(/^post$/i)).not.toBeInTheDocument();
  expect(screen.getByText('core contributors')).toHaveClass('post-meta-chip');
  expect(screen.getByTestId('post-1-author-avatar')).toHaveTextContent('A');
});

test('post card renders the author image when one is available', () => {
  render(
    <PostCard
      view={createView({ authorPicture: 'https://example.com/avatar.png' })}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  expect(screen.getByTestId('post-1-author-avatar').querySelector('img')).toHaveAttribute(
    'src',
    'https://example.com/avatar.png'
  );
});

test('post card renders repost source context for quote reposts', () => {
  render(
    <PostCard
      view={createView({
        post: {
          ...createView().post,
          object_kind: 'repost',
          content: 'adding context',
          repost_commentary: 'adding context',
          repost_of: {
            source_object_id: 'source-1',
            source_topic_id: 'kukuri:topic:source',
            source_author_pubkey: 'b'.repeat(64),
            source_author_display_name: 'Source Author',
            source_author_name: null,
            source_object_kind: 'post',
            content: 'original body',
            attachments: [],
            reply_to: null,
            root_id: 'source-1',
          },
        },
      })}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  expect(screen.getByText('Quote repost')).toBeInTheDocument();
  expect(screen.getByText('Source Author')).toBeInTheDocument();
  expect(screen.getByText('original body')).toBeInTheDocument();
});

test('simple repost opens the source thread in its published topic', async () => {
  const user = userEvent.setup();
  const onOpenThread = vi.fn();
  const onOpenThreadInTopic = vi.fn();

  render(
    <PostCard
      view={createView({
        canReply: false,
        threadTargetId: 'source-root',
        threadTopicId: 'kukuri:topic:source',
        post: {
          ...createView().post,
          object_kind: 'repost',
          content: '',
          repost_commentary: null,
          repost_of: {
            source_object_id: 'source-1',
            source_topic_id: 'kukuri:topic:source',
            source_author_pubkey: 'b'.repeat(64),
            source_author_display_name: 'Source Author',
            source_author_name: null,
            source_object_kind: 'post',
            content: 'original body',
            attachments: [],
            reply_to: null,
            root_id: 'source-root',
          },
        },
      })}
      onOpenAuthor={() => undefined}
      onOpenThread={onOpenThread}
      onOpenThreadInTopic={onOpenThreadInTopic}
      onReply={() => undefined}
    />
  );

  await user.click(screen.getByRole('button', { name: /Source Author/i }));

  expect(onOpenThread).not.toHaveBeenCalled();
  expect(onOpenThreadInTopic).toHaveBeenCalledWith('source-root', 'kukuri:topic:source');
});

test('post card toggles reaction summary chips and opens the reaction tray', async () => {
  const user = userEvent.setup();
  const onToggleReaction = vi.fn();
  const onBookmarkCustomReaction = vi.fn();
  const onManageReactions = vi.fn();
  const customAsset = {
    asset_id: 'asset-1',
    owner_pubkey: 'b'.repeat(64),
    blob_hash: 'blob-1',
    mime: 'image/png',
    bytes: 128,
    width: 128,
    height: 128,
  };
  const view = createView({
    post: {
      ...createView().post,
      reaction_summary: [
        {
          reaction_key_kind: 'emoji',
          normalized_reaction_key: 'emoji:👍',
          emoji: '👍',
          custom_asset: null,
          count: 2,
        },
        {
          reaction_key_kind: 'custom_asset',
          normalized_reaction_key: 'custom_asset:asset-1',
          emoji: null,
          custom_asset: customAsset,
          count: 1,
        },
      ],
      my_reactions: [
        {
          reaction_key_kind: 'emoji',
          normalized_reaction_key: 'emoji:👍',
          emoji: '👍',
          custom_asset: null,
        },
      ],
    },
  });

  render(
    <PostCard
      view={view}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
      localAuthorPubkey={'a'.repeat(64)}
      mediaObjectUrls={{ 'blob-1': 'https://example.com/reaction.png' }}
      onToggleReaction={onToggleReaction}
      onBookmarkCustomReaction={onBookmarkCustomReaction}
      onManageReactions={onManageReactions}
    />
  );

  await user.click(screen.getAllByRole('button', { name: /👍/ })[0]);
  expect(onToggleReaction).toHaveBeenNthCalledWith(1, view.post, { kind: 'emoji', emoji: '👍' });

  await user.click(screen.getByRole('button', { name: 'Save' }));
  expect(onBookmarkCustomReaction).toHaveBeenCalledWith(customAsset);

  await user.click(screen.getByRole('button', { name: 'React' }));
  await user.click(screen.getByRole('button', { name: '🔥' }));
  expect(onToggleReaction).toHaveBeenNthCalledWith(2, view.post, { kind: 'emoji', emoji: '🔥' });

  await user.click(screen.getByRole('button', { name: 'Manage reactions' }));
  expect(onManageReactions).toHaveBeenCalledTimes(1);
});

test('read-only post card hides reaction affordances and keeps the original topic action', async () => {
  const user = userEvent.setup();
  const onOpenOriginalTopic = vi.fn();

  render(
    <PostCard
      view={createView({
        post: {
          ...createView().post,
          published_topic_id: 'kukuri:topic:source',
          reaction_summary: [
            {
              reaction_key_kind: 'emoji',
              normalized_reaction_key: 'emoji:👍',
              emoji: '👍',
              custom_asset: null,
              count: 3,
            },
          ],
        },
      })}
      readOnly
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
      onOpenOriginalTopic={onOpenOriginalTopic}
    />
  );

  expect(screen.queryByRole('button', { name: 'React' })).not.toBeInTheDocument();
  expect(screen.queryByText('3')).not.toBeInTheDocument();

  await user.click(screen.getByRole('button', { name: 'Open original topic' }));
  expect(onOpenOriginalTopic).toHaveBeenCalledWith('kukuri:topic:source');
});
