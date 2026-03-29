import type { Meta, StoryObj } from '@storybook/react-vite';

import { createStoryTopicItems } from '@/components/storyFixtures';
import { Card } from '@/components/ui/card';

import { TopicNavList } from './TopicNavList';

const topicItems = createStoryTopicItems();

const meta = {
  title: 'Core/TopicNavList',
  component: TopicNavList,
  args: {
    items: topicItems,
    onSelectTopic: () => undefined,
    onSelectChannel: () => undefined,
    onRemoveTopic: () => undefined,
  },
  render: () => (
    <div style={{ maxWidth: '320px' }}>
      <Card className='topic-list'>
        <TopicNavList
          items={topicItems}
          onSelectTopic={() => undefined}
          onSelectChannel={() => undefined}
          onRemoveTopic={() => undefined}
        />
      </Card>
    </div>
  ),
} satisfies Meta<typeof TopicNavList>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    onSelectChannel: () => undefined,
  },
};
