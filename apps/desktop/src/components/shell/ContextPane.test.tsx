import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { ContextPane } from './ContextPane';

test('context pane uses the pane title as the only visible header label', () => {
  render(
    <ContextPane paneId='context-pane' title='Author' summary='bob' onClose={vi.fn()}>
      <div>detail body</div>
    </ContextPane>
  );

  expect(screen.getByRole('complementary', { name: 'Author' })).toBeInTheDocument();
  expect(screen.getByText('Author', { selector: 'p' })).toBeInTheDocument();
  expect(screen.getByRole('button', { name: 'Close Author' })).toHaveClass('shell-icon-button');
  expect(screen.queryByText('Context')).not.toBeInTheDocument();
  expect(screen.queryByRole('heading', { name: 'Author' })).not.toBeInTheDocument();
  expect(screen.queryByText('bob')).not.toBeInTheDocument();
});
