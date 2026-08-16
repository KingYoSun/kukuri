import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
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
      onToggleMute={vi.fn()}
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
  expect(screen.getByRole('button', { name: 'Unfollow' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Mute' })).toBeInTheDocument();
  expect(screen.getByText('mutual').closest('.author-detail-identity')).toContainElement(
    screen.getByText('bob')
  );
  expect(screen.getByText('mutual').closest('.author-detail-actions')).toBeNull();
});

test('author report does not fetch a manifest or create a candidate without provenance', async () => {
  const user = userEvent.setup();
  const onFetchReportManifest = vi.fn();

  render(
    <AuthorDetailCard
      view={{
        ...STORY_AUTHOR_DETAIL_VIEW,
        author: {
          ...STORY_AUTHOR_DETAIL_VIEW.author!,
          provenance: null,
        },
      }}
      localAuthorPubkey={'f'.repeat(64)}
      onToggleRelationship={vi.fn()}
      onToggleMute={vi.fn()}
      onSubmitReport={vi.fn()}
      onFetchReportManifest={onFetchReportManifest}
    />
  );

  await user.click(screen.getByRole('button', { name: 'Report' }));

  expect(screen.getByRole('dialog')).toBeInTheDocument();
  expect(onFetchReportManifest).not.toHaveBeenCalled();
  expect(screen.getByText('Cannot determine a report target')).toBeInTheDocument();
  expect(screen.getByText(/block, mute, or hide this locally/i)).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
});
