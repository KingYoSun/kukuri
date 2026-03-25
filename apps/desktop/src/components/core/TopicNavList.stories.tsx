import type { Meta, StoryObj } from '@storybook/react-vite';

import { STORY_TOPIC_ITEMS } from '@/components/storyFixtures';
import { Card } from '@/components/ui/card';

import { TopicNavList } from './TopicNavList';

const meta = {
  title: 'Core/TopicNavList',
  component: TopicNavList,
  args: {
    items: STORY_TOPIC_ITEMS,
    onSelectTopic: () => undefined,
    onRemoveTopic: () => undefined,
  },
  render: () => (
    <div style={{ maxWidth: '320px' }}>
      <Card className='topic-list'>
        <TopicNavList
          items={STORY_TOPIC_ITEMS}
          onSelectTopic={() => undefined}
          onRemoveTopic={() => undefined}
        />
      </Card>
    </div>
  ),
} satisfies Meta<typeof TopicNavList>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
