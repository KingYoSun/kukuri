import type { Meta, StoryObj } from '@storybook/react-vite';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { STORY_AUTHOR_DETAIL_VIEW } from '@/components/storyFixtures';

import { ContextPane } from './ContextPane';

const meta = {
  title: 'Shell/ContextPane',
  component: ContextPane,
  parameters: {
    layout: 'padded',
  },
  args: {
    paneId: 'storybook-shell-context',
    title: 'Author',
    summary: 'bob',
    showBackdrop: true,
    onClose: () => undefined,
    children: (
      <AuthorDetailCard
        view={STORY_AUTHOR_DETAIL_VIEW}
        localAuthorPubkey={'f'.repeat(64)}
        onToggleRelationship={() => undefined}
      />
    ),
  },
} satisfies Meta<typeof ContextPane>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
