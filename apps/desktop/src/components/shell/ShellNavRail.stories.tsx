import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { Settings } from 'lucide-react';

import { TopicNavList } from '@/components/core/TopicNavList';
import { createStoryTopicItems } from '@/components/storyFixtures';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import i18n from '@/i18n';

import { ShellNavRail } from './ShellNavRail';

function ShellNavRailStory() {
  const [open, setOpen] = useState(true);
  const [topicInput, setTopicInput] = useState('kukuri:topic:phase5');
  const topicItems = createStoryTopicItems();

  return (
    <div style={{ maxWidth: '320px' }}>
      <ShellNavRail
        railId='storybook-shell-nav'
        open={open}
        onOpenChange={setOpen}
        headerContent={
          <div className='shell-nav-status'>
            <div className='shell-status-badges'>
              <StatusBadge label={i18n.t('common:states.connected')} tone='accent' />
              <StatusBadge label='2 peers' />
              <StatusBadge label={i18n.t('shell:navigation.seededDht')} />
            </div>
            <Button className='shell-settings-button' variant='ghost' size='icon' type='button'>
              <Settings className='size-6' aria-hidden='true' />
            </Button>
          </div>
        }
        addTopicControl={
          <Label>
            <span>{i18n.t('shell:navigation.addTopic')}</span>
            <div className='topic-input-row'>
              <Input
                value={topicInput}
                onChange={(event) => setTopicInput(event.target.value)}
                placeholder={i18n.t('shell:navigation.placeholder')}
              />
              <Button variant='secondary' type='button'>
                {i18n.t('common:actions.add')}
              </Button>
            </div>
          </Label>
        }
        topicList={
          <TopicNavList
            items={topicItems}
            onSelectTopic={() => undefined}
            onRemoveTopic={() => undefined}
          />
        }
        topicCount={topicItems.length}
      />
    </div>
  );
}

const topicItems = createStoryTopicItems();

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
    topicCount: topicItems.length,
  },
  render: () => <ShellNavRailStory />,
} satisfies Meta<typeof ShellNavRail>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
};
