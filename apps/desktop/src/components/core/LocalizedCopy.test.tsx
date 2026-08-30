import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, test } from 'vitest';

import i18n from '@/i18n';

import { PostCard } from './PostCard';
import { createView } from './PostCard.testHelpers';
import { ThreadTree } from './ThreadTree';
import { TimelineFeed } from './TimelineFeed';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

describe('Japanese shared post copy', () => {
  test.each([
    ['timeline', TimelineFeed],
    ['thread', ThreadTree],
  ] as const)('%s uses localized manual pagination actions', async (_surface, Component) => {
    await i18n.changeLanguage('ja');

    const { rerender } = render(
      <Component
        posts={[createView()]}
        emptyCopy='空です'
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
        hasMore
        onLoadMore={() => undefined}
      />
    );

    expect(screen.getByRole('button', { name: 'さらに読み込む' })).toBeInTheDocument();

    rerender(
      <Component
        posts={[createView()]}
        emptyCopy='空です'
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
        hasMore
        loadingMore
        onLoadMore={() => undefined}
      />
    );

    expect(screen.getByRole('button', { name: 'さらに読み込み中…' })).toBeInTheDocument();
  });

  test('repost attachment count uses localized media copy', async () => {
    await i18n.changeLanguage('ja');
    const base = createView();
    const attachment = {
      hash: 'b'.repeat(64),
      mime: 'image/png',
      bytes: 2048,
      role: 'image_original',
      status: 'Available' as const,
    };

    render(
      <PostCard
        view={createView({
          canReply: false,
          repostSourceAuthor: { pubkey: 'b'.repeat(64), label: 'Source Author', picture: null },
          post: {
            ...base.post,
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
              attachments: [attachment, { ...attachment, hash: 'c'.repeat(64) }],
              reply_to: null,
              root_id: 'source-root',
            },
          },
        })}
        onOpenAuthor={() => undefined}
        onOpenThread={() => undefined}
        onReply={() => undefined}
      />
    );

    expect(screen.getByText('メディア2件')).toBeInTheDocument();
  });
});
