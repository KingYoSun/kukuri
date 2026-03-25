import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { TopicNavList } from '@/components/core/TopicNavList';
import {
  STORY_PRIMARY_ITEMS,
  STORY_TOPIC_ITEMS,
} from '@/components/storyFixtures';
import type { PrimarySection } from '@/components/shell/types';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

import { ShellNavRail } from './ShellNavRail';

function ShellNavRailStory() {
  const [open, setOpen] = useState(true);
  const [activePrimarySection, setActivePrimarySection] = useState<PrimarySection>('timeline');
  const [topicInput, setTopicInput] = useState('kukuri:topic:phase5');

  return (
    <div style={{ maxWidth: '320px' }}>
      <ShellNavRail
        railId='storybook-shell-nav'
        open={open}
        onOpenChange={setOpen}
        primaryItems={STORY_PRIMARY_ITEMS}
        activePrimarySection={activePrimarySection}
        onSelectPrimarySection={setActivePrimarySection}
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
    primaryItems: STORY_PRIMARY_ITEMS,
    activePrimarySection: 'timeline',
    onSelectPrimarySection: () => undefined,
    addTopicControl: null,
    topicList: null,
    topicCount: STORY_TOPIC_ITEMS.length,
  },
  render: () => <ShellNavRailStory />,
} satisfies Meta<typeof ShellNavRail>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};
