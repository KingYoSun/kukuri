import type { Meta, StoryObj } from '@storybook/react-vite';
import { PanelLeftOpen, Settings } from 'lucide-react';

import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TopicNavList } from '@/components/core/TopicNavList';
import { createStoryTopicItems } from '@/components/storyFixtures';
import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import i18n from '@/i18n';

import { ContextPane } from './ContextPane';
import { ShellFrame } from './ShellFrame';
import { ShellNavRail } from './ShellNavRail';
import { ShellTopBar } from './ShellTopBar';

const meta = {
  title: 'Shell/ShellFrame',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

function ShellFrameStory() {
  const topicItems = createStoryTopicItems();
  return (
    <div style={{ width: '1440px', minWidth: '1440px', margin: '0 auto' }}>
      <ShellFrame
        skipTargetId='storybook-shell-workspace'
        topBar={<ShellTopBar activeTopic='kukuri:topic:demo' />}
        navRail={
          <ShellNavRail
            railId='storybook-shell-nav'
            open={true}
            onOpenChange={() => undefined}
            headerContent={
              <div className='shell-nav-status'>
                <div className='shell-status-badges'>
                  <StatusBadge label={i18n.t('common:states.connected')} tone='accent' />
                  <StatusBadge label='2 peers' />
                  <StatusBadge label={i18n.t('shell:navigation.seededDht')} />
                </div>
                <Button variant='ghost' size='icon' type='button'>
                  <Settings className='size-5' aria-hidden='true' />
                </Button>
              </div>
            }
            addTopicControl={
              <Label>
                <span>{i18n.t('shell:navigation.addTopic')}</span>
                <div className='topic-input-row'>
                  <Input value='kukuri:topic:demo' onChange={() => undefined} />
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
        }
        workspace={
          <div className='shell-main-stack'>
            <Card className='shell-workspace-card shell-workspace-header-card'>
              <TimelineWorkspaceHeader
                activeSection='timeline'
                items={[
                  { id: 'timeline', label: i18n.t('shell:primarySections.timeline') },
                  { id: 'channels', label: i18n.t('shell:primarySections.channels') },
                  { id: 'live', label: i18n.t('shell:primarySections.live') },
                  { id: 'game', label: i18n.t('shell:primarySections.game') },
                  { id: 'profile', label: i18n.t('shell:primarySections.profile') },
                ]}
                onSelectSection={() => undefined}
              />
            </Card>
            <Card className='shell-workspace-card'>
              <h3>Workspace Input</h3>
              <p className='lede'>Active workspace input lives here.</p>
            </Card>
            <Card className='shell-workspace-card'>
              <h3>Workspace List</h3>
              <p className='lede'>Synchronized items render here.</p>
            </Card>
          </div>
        }
        detailPaneStack={
          <ContextPane
            paneId='storybook-shell-thread'
            title={i18n.t('shell:context.thread')}
            summary={i18n.t('shell:context.threadSummary', { count: 2 })}
            onClose={() => undefined}
          >
            <Card>
              <p className='lede'>Thread detail pane</p>
            </Card>
          </ContextPane>
        }
        detailPaneCount={1}
        mobileFooter={
          <Button variant='secondary' type='button'>
            <PanelLeftOpen className='size-5' aria-hidden='true' />
            {i18n.t('shell:navigation.topicsButton')}
          </Button>
        }
      />
    </div>
  );
}

export const Default: Story = {
  render: () => <ShellFrameStory />,
};
