import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { Settings } from 'lucide-react';

import { TopicNavList } from '@/components/core/TopicNavList';
import { STORY_TOPIC_ITEMS } from '@/components/storyFixtures';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { ShellNavRail } from './ShellNavRail';

function ShellNavRailStory() {
  const [open, setOpen] = useState(true);
  const [topicInput, setTopicInput] = useState('kukuri:topic:phase5');

  return (
    <div style={{ maxWidth: '320px' }}>
      <ShellNavRail
        railId='storybook-shell-nav'
        open={open}
        onOpenChange={setOpen}
        headerContent={
          <div className='shell-nav-status'>
            <div className='shell-status-badges'>
              <StatusBadge label='connected' tone='accent' />
              <StatusBadge label='2 peers' />
              <StatusBadge label='seeded dht' />
            </div>
            <Button className='shell-settings-button' variant='ghost' size='icon' type='button'>
              <Settings className='size-6' aria-hidden='true' />
            </Button>
          </div>
        }
        addTopicControl={
          <Label>
            <span>Add Topic</span>
            <div className='topic-input-row'>
              <Input
                value={topicInput}
                onChange={(event) => setTopicInput(event.target.value)}
                placeholder='kukuri:topic:demo'
              />
              <Button variant='secondary' type='button'>
                Add
              </Button>
            </div>
          </Label>
        }
        topicList={
          <TopicNavList
            items={STORY_TOPIC_ITEMS}
            onSelectTopic={() => undefined}
            onRemoveTopic={() => undefined}
          />
        }
        topicCount={STORY_TOPIC_ITEMS.length}
      />
    </div>
  );
}

const meta = {
  title: 'Shell/ShellNavRail',
  component: ShellNavRail,
  parameters: {
    layout: 'padded',
  },
  args: {
    railId: 'storybook-shell-nav',
    open: true,
    onOpenChange: () => undefined,
    headerContent: null,
    addTopicControl: null,
    topicList: null,
    topicCount: STORY_TOPIC_ITEMS.length,
  },
  render: () => <ShellNavRailStory />,
} satisfies Meta<typeof ShellNavRail>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
