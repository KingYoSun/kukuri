import type { Meta, StoryObj } from '@storybook/react-vite';

import {
  createStoryAuthorDetailView,
  STORY_EMPTY_AUTHOR_DETAIL_VIEW,
} from '@/components/storyFixtures';

import { AuthorDetailCard } from './AuthorDetailCard';

const authorDetailView = createStoryAuthorDetailView();

const meta = {
  title: 'Core/AuthorDetailCard',
  component: AuthorDetailCard,
  render: (args) => (
    <div style={{ maxWidth: '420px' }}>
      <AuthorDetailCard {...args} />
    </div>
  ),
  args: {
    view: authorDetailView,
    localAuthorPubkey: 'f'.repeat(64),
    onToggleRelationship: () => undefined,
  },
} satisfies Meta<typeof AuthorDetailCard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Selected: Story = {};

export const Empty: Story = {
  args: {
    view: STORY_EMPTY_AUTHOR_DETAIL_VIEW,
  },
};
