import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';
import { ProfileOverviewPanel } from './ProfileOverviewPanel';

const props = {
  authorLabel: 'Display Name', username: 'username', about: null, picture: null,
  status: 'ready' as const, error: null, postCount: 0,
  followingCount: 0, followedCount: 0, mutedCount: 0, blockingCount: 0,
  onEdit: vi.fn(), onOpenFollowing: vi.fn(), onOpenFollowed: vi.fn(), onOpenMuted: vi.fn(), onOpenBlocking: vi.fn(),
};

test('displays the name and username separately without a redundant Profile heading', () => {
  render(<ProfileOverviewPanel {...props} />);
  expect(screen.getByRole('heading', { name: 'Display Name' })).toBeInTheDocument();
  expect(screen.getByText('username')).toBeInTheDocument();
  expect(screen.queryByRole('heading', { name: 'Profile' })).not.toBeInTheDocument();
});

test.each([null, '', '   '])('keeps a username line for an unset value %s', (username) => {
  render(<ProfileOverviewPanel {...props} username={username} />);
  expect(screen.getByText('Username not set')).toBeInTheDocument();
});
