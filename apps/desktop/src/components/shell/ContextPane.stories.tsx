import { useMemo, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import {
  STORY_AUTHOR_DETAIL_VIEW,
  STORY_THREAD_PANEL_STATE,
  STORY_THREAD_POSTS,
} from '@/components/storyFixtures';

import { ContextPane } from './ContextPane';

function ContextPaneStory() {
  const [open, setOpen] = useState(true);
  const [activeMode, setActiveMode] = useState<'thread' | 'author'>('thread');
  const tabs = useMemo(
    () => [
      {
        id: 'thread' as const,
        label: 'Thread',
        summary: '2 posts in the active thread',
        content: (
          <ThreadPanel
            state={STORY_THREAD_PANEL_STATE}
            posts={STORY_THREAD_POSTS}
            onClearThread={() => undefined}
            onOpenAuthor={() => undefined}
            onOpenThread={() => undefined}
            onReply={() => undefined}
          />
        ),
      },
      {
        id: 'author' as const,
        label: 'Author',
        summary: 'bob',
        content: (
          <AuthorDetailCard
            view={STORY_AUTHOR_DETAIL_VIEW}
            localAuthorPubkey={'f'.repeat(64)}
            onClearAuthor={() => undefined}
            onToggleRelationship={() => undefined}
          />
        ),
      },
    ],
    []
  );

  return (
    <div style={{ maxWidth: '380px' }}>
      <ContextPane
        paneId='storybook-shell-context'
        open={open}
        onOpenChange={setOpen}
        activeMode={activeMode}
        onModeChange={setActiveMode}
        tabs={tabs}
      />
    </div>
  );
}

const meta = {
  title: 'Shell/ContextPane',
  component: ContextPane,
  parameters: {
    layout: 'padded',
  },
  args: {
    paneId: 'storybook-shell-context',
    open: true,
    onOpenChange: () => undefined,
    activeMode: 'thread',
    onModeChange: () => undefined,
    tabs: [],
  },
  render: () => <ContextPaneStory />,
} satisfies Meta<typeof ContextPane>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
