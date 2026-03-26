import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { STORY_AUTHOR_DETAIL_VIEW } from '@/components/storyFixtures';

import { AuthorDetailCard } from './AuthorDetailCard';

test('author detail marks long unbroken values as wrappable content', () => {
  const longPubkey = 'b'.repeat(96);
  const longViaPubkey = 'c'.repeat(96);
  const longAbout = `Maintains ${'connectivity'.repeat(12)}`;

  render(
    <AuthorDetailCard
      view={{
        ...STORY_AUTHOR_DETAIL_VIEW,
        author: {
          ...STORY_AUTHOR_DETAIL_VIEW.author!,
          author_pubkey: longPubkey,
          about: longAbout,
        },
        summary: {
          ...STORY_AUTHOR_DETAIL_VIEW.summary!,
          viaPubkeys: [longViaPubkey],
        },
      }}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
    />
  );

  expect(screen.queryByText('Author Detail')).not.toBeInTheDocument();
  expect(screen.getByTestId('author-detail-avatar')).toHaveClass('author-avatar-sm');
  expect(screen.getByTestId('author-detail-avatar')).toHaveTextContent('B');
  expect(screen.queryByRole('button', { name: 'Clear author' })).not.toBeInTheDocument();
  expect(screen.getByText('bob')).toHaveClass('author-detail-break');
  expect(screen.getByText(longAbout)).toHaveClass('author-detail-break');
  expect(screen.getByText(longAbout).parentElement).toHaveClass('author-detail-copy-stack');
  expect(screen.getByText(longPubkey)).toHaveClass('author-detail-monotext');
  expect(screen.getByText(longViaPubkey)).toHaveClass('author-detail-break');
  expect(screen.queryByText('following: yes')).not.toBeInTheDocument();
  expect(screen.queryByText('followed by: yes')).not.toBeInTheDocument();
  const followButton = screen.getByRole('button', { name: 'Unfollow' });
  expect(screen.getByText('mutual').closest('.author-detail-actions')).toContainElement(followButton);
});
