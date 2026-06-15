import type { Meta, StoryObj } from '@storybook/react-vite';

import { createStoryTopicItems } from '@/components/storyFixtures';
import { Card } from '@/components/ui/card';

import { TopicNavList } from './TopicNavList';
import { type TopicDiagnosticSummary } from './types';

const baseItems = createStoryTopicItems();

const topicItems: TopicDiagnosticSummary[] = baseItems.map((item, index) =>
  index === 0
    ? {
        ...item,
        gossipJoined: true,
        channels: [
          {
            channelId: 'channel-core',
            label: 'Core',
            audienceKind: 'friend_plus',
            active: false,
            gossipJoined: true,
          },
          {
            channelId: 'channel-muted',
            label: 'Archive',
            audienceKind: 'invite_only',
            active: false,
            gossipJoined: false,
          },
        ],
      }
    : { ...item, gossipJoined: false }
);

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
          onOpenChannelSettings={() => undefined}
          onLeaveChannel={() => undefined}
          onRemoveTopic={() => undefined}
          onCopyTopicLink={() => undefined}
          onToggleTopicGossip={() => undefined}
          onToggleChannelGossip={() => undefined}
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
