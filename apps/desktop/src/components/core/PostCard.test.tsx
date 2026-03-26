import { render, screen } from '@testing-library/react';
import { expect, test } from 'vitest';

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
      channel_id: null,
      audience_label: 'Public',
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
