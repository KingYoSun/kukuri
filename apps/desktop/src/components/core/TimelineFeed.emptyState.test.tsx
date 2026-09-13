// #994: 空状態の描画は呼出し側が差し替えられる。差し替えない caller は従来の 1 行文言のまま(INVAR-2)。
import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { TimelineFeed } from './TimelineFeed';

const baseProps = {
  posts: [],
  emptyCopy: 'No posts in this topic yet.',
  onOpenAuthor: vi.fn(),
  onOpenThread: vi.fn(),
  onReply: vi.fn(),
};

test('renders the plain empty copy when no custom empty state is given', () => {
  render(<TimelineFeed {...baseProps} />);
  expect(screen.getByText('No posts in this topic yet.')).toHaveClass('empty');
});

test('renders the custom empty state instead of the plain copy', () => {
  render(
    <TimelineFeed
      {...baseProps}
      emptyState={<div data-testid='custom-empty'>guidance</div>}
    />
  );
  expect(screen.getByTestId('custom-empty')).toBeInTheDocument();
  expect(screen.queryByText('No posts in this topic yet.')).not.toBeInTheDocument();
});

test('renders nothing for the empty branch when the custom empty state is null (loading / error owner renders the notice)', () => {
  const { container } = render(<TimelineFeed {...baseProps} emptyState={null} />);
  expect(screen.queryByText('No posts in this topic yet.')).not.toBeInTheDocument();
  expect(container.querySelector('.empty')).toBeNull();
});
