import { render, screen } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import { STORY_THREAD_PANEL_STATE, STORY_THREAD_POSTS } from '@/components/storyFixtures';

import { ThreadPanel } from './ThreadPanel';

test('thread panel renders only the thread feed and relies on the pane header for close affordance', () => {
  render(
    <ThreadPanel
      state={STORY_THREAD_PANEL_STATE}
      posts={STORY_THREAD_POSTS}
      onOpenAuthor={vi.fn()}
      onOpenThread={vi.fn()}
      onReply={vi.fn()}
    />
  );

  expect(screen.queryByRole('heading', { name: 'Thread' })).not.toBeInTheDocument();
  expect(screen.queryByText(STORY_THREAD_PANEL_STATE.summary)).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Clear thread' })).not.toBeInTheDocument();
  expect(screen.getByText(STORY_THREAD_POSTS[0].post.content)).toBeInTheDocument();
});
