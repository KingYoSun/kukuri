import type { Meta, StoryObj } from '@storybook/react-vite';

import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { createStoryAuthorDetailView } from '@/components/storyFixtures';
import i18n from '@/i18n';

import { ContextPane } from './ContextPane';

const authorDetailView = createStoryAuthorDetailView();

const meta = {
  title: 'Shell/ContextPane',
  component: ContextPane,
  parameters: {
    layout: 'padded',
  },
  args: {
    paneId: 'storybook-shell-context',
    title: i18n.t('shell:context.author'),
    summary: 'bob',
    showBackdrop: true,
    onClose: () => undefined,
    children: (
      <AuthorDetailCard
        view={authorDetailView}
        localAuthorPubkey={'f'.repeat(64)}
        onToggleRelationship={() => undefined}
        onToggleMute={() => undefined}
      />
    ),
  },
} satisfies Meta<typeof ContextPane>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
