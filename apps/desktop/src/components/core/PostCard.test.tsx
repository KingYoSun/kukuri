import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
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

function setViewportWidth(width: number) {
  Object.defineProperty(window, 'innerWidth', {
    configurable: true,
    writable: true,
    value: width,
  });
  window.dispatchEvent(new Event('resize'));
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

test('clicking the author avatar triggers the same author action as the name', async () => {
  const user = userEvent.setup();
  const onOpenAuthor = vi.fn();

  render(
    <PostCard
      view={createView()}
      onOpenAuthor={onOpenAuthor}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  await user.click(screen.getByTestId('post-1-author-avatar'));

  expect(onOpenAuthor).toHaveBeenCalledWith('a'.repeat(64));
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

test('post card renders reply parent preview inline', () => {
  render(
    <PostCard
      view={createView({
        post: {
          ...createView().post,
          reply_to: 'parent-1',
          reply_preview: {
            object_id: 'parent-1',
            topic: 'kukuri:topic:source',
            author: {
              pubkey: 'b'.repeat(64),
              name: 'parent-author',
              display_name: 'Parent Author',
              picture: null,
              picture_asset: null,
            },
            content: 'parent body',
            attachments: [],
            root_id: 'parent-1',
            reply_to: null,
          },
        },
      })}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  expect(screen.getByText('Parent Author')).toBeInTheDocument();
  expect(screen.getByText('parent body')).toBeInTheDocument();
  expect(screen.getAllByText('Reply').length).toBeGreaterThan(0);
});

test('post card marks long content fields as wrap-safe', () => {
  const longContent = 'channel_payload_'.repeat(48);
  const longEnvelopeId = 'f'.repeat(192);

  render(
    <PostCard
      view={createView({
        post: {
          ...createView().post,
          content: longContent,
          envelope_id: longEnvelopeId,
        },
      })}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  expect(screen.getByText(longContent)).toHaveClass('post-copy-wrap');
  expect(screen.getByText(longEnvelopeId)).toHaveClass('post-copy-wrap');
});

test('post card opens a media dialog and navigates multi-image attachments', async () => {
  const user = userEvent.setup();

  render(
    <PostCard
      view={createView({
        media: {
          ...createView().media,
          kind: 'image',
          imagePreviewSrc: 'https://example.com/one.png',
          imageGalleryItems: [
            {
              hash: 'image-1',
              src: 'https://example.com/one.png',
              mime: 'image/png',
            },
            {
              hash: 'image-2',
              src: 'https://example.com/two.png',
              mime: 'image/png',
            },
          ],
          currentImageIndex: 0,
        },
      })}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
    />
  );

  await user.click(screen.getByRole('button', { name: 'image attachment' }));

  const dialog = screen.getByRole('dialog');
  expect(dialog).toHaveClass('media-viewer-dialog');
  expect(dialog.querySelector('.media-viewer-counter')).toBeNull();
  expect(within(dialog).getByRole('img', { name: 'image attachment' })).toHaveAttribute(
    'src',
    'https://example.com/one.png'
  );

  await user.click(within(dialog).getByRole('button', { name: 'Next image' }));

  expect(within(dialog).getByRole('img', { name: 'image attachment' })).toHaveAttribute(
    'src',
    'https://example.com/two.png'
  );
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

test('post card opens a custom reaction context menu and keeps the reaction popover search flow', async () => {
  setViewportWidth(280);
  const user = userEvent.setup();
  const onToggleReaction = vi.fn();
  const onBookmarkCustomReaction = vi.fn();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: {
      writeText: clipboardWriteText,
    },
  });
  const customAsset = {
    asset_id: 'parrot-asset',
    owner_pubkey: 'b'.repeat(64),
    blob_hash: 'blob-1',
    search_key: 'party-parrot',
    mime: 'image/png',
    bytes: 128,
    width: 128,
    height: 128,
  };
  const bookmarkedAsset = {
    asset_id: 'asset-2',
    owner_pubkey: 'c'.repeat(64),
    blob_hash: 'blob-2',
    search_key: 'saved-cat',
    mime: 'image/gif',
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
          normalized_reaction_key: 'custom_asset:parrot-asset',
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
      mediaObjectUrls={{
        'blob-1': 'https://example.com/reaction.png',
        'blob-2': 'https://example.com/bookmarked.png',
      }}
      bookmarkedReactionAssets={[bookmarkedAsset]}
      recentReactions={[
        {
          reaction_key_kind: 'emoji',
          normalized_reaction_key: 'emoji:🔥',
          emoji: '🔥',
          custom_asset: null,
          updated_at: 2,
        },
      ]}
      onToggleReaction={onToggleReaction}
      onBookmarkCustomReaction={onBookmarkCustomReaction}
    />
  );

  expect(screen.queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();

  await user.click(screen.getAllByRole('button', { name: /👍/ })[0]);
  expect(onToggleReaction).toHaveBeenNthCalledWith(1, view.post, { kind: 'emoji', emoji: '👍' });

  const customReactionChip = screen.getByAltText(customAsset.asset_id).closest('button');
  if (!(customReactionChip instanceof HTMLButtonElement)) {
    throw new Error('custom reaction chip not found');
  }

  fireEvent.contextMenu(customReactionChip);
  await user.click(screen.getByRole('menuitem', { name: 'Copy hash' }));
  expect(clipboardWriteText).toHaveBeenCalledWith(customAsset.blob_hash);
  expect(onToggleReaction).toHaveBeenCalledTimes(1);

  fireEvent.contextMenu(customReactionChip);
  await user.click(screen.getByRole('menuitem', { name: 'Save' }));
  expect(onBookmarkCustomReaction).toHaveBeenCalledWith(customAsset);

  await user.click(screen.getByRole('button', { name: 'React' }));
  expect(screen.queryByRole('button', { name: 'Manage reactions' })).not.toBeInTheDocument();
  expect(screen.getByText('Recent')).toBeInTheDocument();
  expect(screen.getByText('Emoji')).toBeInTheDocument();
  expect(screen.getByText('Custom')).toBeInTheDocument();
  const reactionPopover = screen.getByPlaceholderText('Search reactions').closest('.post-reaction-popover');
  expect(reactionPopover).toHaveClass('post-reaction-popover-wide');
  expect(reactionPopover).not.toHaveClass('post-action-popover');
  expect(reactionPopover).toHaveStyle({ width: '248px' });
  expect(reactionPopover).toHaveStyle({ '--reaction-grid-columns': '6' });
  expect(screen.queryByText(bookmarkedAsset.asset_id)).not.toBeInTheDocument();
  const emojiSection = screen.getByText('Emoji').closest('section');
  if (!(emojiSection instanceof HTMLElement)) {
    throw new Error('emoji section not found');
  }
  expect(emojiSection.querySelector('.post-reaction-picker-grid-8')).not.toBeNull();
  expect(
    within(emojiSection).getByRole('button', { name: 'thumbs-up' })
  ).toHaveAttribute('data-tooltip', 'thumbs-up');
  expect(screen.getByRole('button', { name: /saved-cat/i })).toHaveAttribute(
    'data-tooltip',
    'saved-cat'
  );
  setViewportWidth(1024);
  await waitFor(() => {
    expect(reactionPopover).toHaveStyle({ width: '410px' });
    expect(reactionPopover).toHaveStyle({ '--reaction-grid-columns': '8' });
  });
  await user.click(
    within(screen.getByText('Recent').closest('section') as HTMLElement).getByRole('button', {
      name: 'fire',
    })
  );
  expect(onToggleReaction).toHaveBeenNthCalledWith(2, view.post, { kind: 'emoji', emoji: '🔥' });

  await user.click(screen.getByRole('button', { name: 'React' }));
  await user.type(screen.getByPlaceholderText('Search reactions'), 'saved');
  await user.click(screen.getByRole('button', { name: /saved-cat/i }));
  expect(onToggleReaction).toHaveBeenNthCalledWith(3, view.post, {
    kind: 'custom_asset',
    asset: bookmarkedAsset,
  });
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

test('post card renders bookmark as an icon-only action with an accessible label', async () => {
  const user = userEvent.setup();
  const onToggleBookmark = vi.fn();

  render(
    <PostCard
      view={createView()}
      onOpenAuthor={() => undefined}
      onOpenThread={() => undefined}
      onReply={() => undefined}
      showBookmarkAction
      isBookmarked
      onToggleBookmark={onToggleBookmark}
    />
  );

  const bookmarkButton = screen.getByRole('button', { name: 'Remove bookmark' });
  expect(bookmarkButton).toHaveAttribute('aria-pressed', 'true');
  expect(bookmarkButton).toHaveClass('post-action-button-active');
  expect(bookmarkButton).not.toHaveTextContent(/bookmark/i);

  await user.click(bookmarkButton);
  expect(onToggleBookmark).toHaveBeenCalledWith(createView().post);
});
