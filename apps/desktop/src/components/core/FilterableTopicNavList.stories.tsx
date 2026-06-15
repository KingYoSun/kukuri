import type { Meta, StoryObj } from '@storybook/react-vite';

import { createStoryTopicItems } from '@/components/storyFixtures';
import { Card } from '@/components/ui/card';

import { FilterableTopicNavList } from './FilterableTopicNavList';
import { type TopicDiagnosticSummary } from './types';

const baseItems = createStoryTopicItems();

const topicItems: TopicDiagnosticSummary[] = baseItems.map((item, index) =>
  index === 0
    ? {
        ...item,
        gossipJoined: true,
        lastReceivedAt: 1_711_629_911_000,
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
    : { ...item, gossipJoined: false, lastReceivedAt: 1_711_543_511_000 }
);

const meta = {
  title: 'Core/FilterableTopicNavList',
  component: FilterableTopicNavList,
  render: () => (
    <div style={{ maxWidth: '320px' }}>
      <Card className='topic-list'>
        <FilterableTopicNavList
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
} satisfies Meta<typeof FilterableTopicNavList>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    items: topicItems,
    onSelectTopic: () => undefined,
    onSelectChannel: () => undefined,
    onRemoveTopic: () => undefined,
  },
};
