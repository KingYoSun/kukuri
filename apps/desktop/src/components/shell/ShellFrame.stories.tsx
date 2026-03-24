import { useMemo, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { StatusBadge } from '@/components/StatusBadge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';

import { ContextPane } from './ContextPane';
import { ShellFrame } from './ShellFrame';
import { ShellNavRail } from './ShellNavRail';
import { SettingsDrawer } from './SettingsDrawer';
import { ShellTopBar } from './ShellTopBar';
import {
  type ContextPaneMode,
  type PrimarySection,
  type SettingsSection,
  type ShellChromeState,
} from './types';

const meta = {
  title: 'Shell/ShellFrame',
  parameters: {
    layout: 'fullscreen',
  },
} satisfies Meta;

export default meta;

type Story = StoryObj<typeof meta>;

type ShellStoryFixtureProps = {
  width: number;
  initialChromeState?: Partial<ShellChromeState>;
};

const PRIMARY_ITEMS: Array<{
  id: PrimarySection;
  label: string;
  description: string;
}> = [
  { id: 'timeline', label: 'Timeline', description: 'Feed and scope controls' },
  { id: 'channels', label: 'Channels', description: 'Private channel entry and composer' },
  { id: 'live', label: 'Live', description: 'Live sessions and status' },
  { id: 'game', label: 'Game', description: 'Scoreboards and room editing' },
  { id: 'profile', label: 'Profile', description: 'Edit author identity' },
];

const SETTINGS_ITEMS: Array<{
  id: SettingsSection;
  label: string;
  description: string;
}> = [
  { id: 'connectivity', label: 'Connectivity', description: 'Peer status and tickets' },
  { id: 'discovery', label: 'Discovery', description: 'Seed configuration and diagnostics' },
  {
    id: 'community-node',
    label: 'Community Node',
    description: 'Auth, consent, and community metadata',
  },
];

function ShellStoryFixture({ width, initialChromeState }: ShellStoryFixtureProps) {
  const [chromeState, setChromeState] = useState<ShellChromeState>({
    activePrimarySection: 'timeline',
    activeContextPaneMode: 'thread',
    activeSettingsSection: 'connectivity',
    navOpen: false,
    contextOpen: false,
    settingsOpen: false,
    ...initialChromeState,
  });

  const contextTabs = useMemo(
    () => [
      {
        id: 'thread' as ContextPaneMode,
        label: 'Thread',
        summary: '3 replies in the active thread',
        content: (
          <div className='shell-main-stack'>
            <Card>
              <h3>Thread</h3>
              <p className='lede'>Keep the reading context visible without taking over the workspace.</p>
            </Card>
          </div>
        ),
      },
      {
        id: 'author' as ContextPaneMode,
        label: 'Author',
        summary: 'alice',
        content: (
          <div className='shell-main-stack'>
            <Card>
              <h3>Author Detail</h3>
              <p className='lede'>Profile, relationship badges, and follow actions live here.</p>
            </Card>
          </div>
        ),
      },
    ],
    []
  );

  const settingsSections = useMemo(
    () =>
      SETTINGS_ITEMS.map((section) => ({
        ...section,
        content: (
          <div className='shell-main-stack'>
            <Card>
              <h3>{section.label}</h3>
              <p className='lede'>{section.description}</p>
              <Notice tone={section.id === 'connectivity' ? 'accent' : 'neutral'}>
                Review surface for {section.label.toLowerCase()}.
              </Notice>
            </Card>
          </div>
        ),
      })),
    []
  );

  return (
    <div style={{ maxWidth: `${width}px`, margin: '0 auto' }}>
      <ShellFrame
        skipTargetId='story-shell-workspace'
        topBar={
          <ShellTopBar
            headline='Seeded DHT + direct peers'
            activeTopic='kukuri:topic:demo'
            statusBadges={
              <>
                <StatusBadge label='connected' tone='accent' />
                <StatusBadge label='1 peers' />
                <StatusBadge label='seeded dht' />
              </>
            }
            navOpen={chromeState.navOpen}
            settingsOpen={chromeState.settingsOpen}
            navControlsId='story-shell-nav'
            settingsControlsId='story-shell-settings'
            onToggleNav={() =>
              setChromeState((current) => ({
                ...current,
                navOpen: !current.navOpen,
              }))
            }
            onToggleSettings={() =>
              setChromeState((current) => ({
                ...current,
                settingsOpen: !current.settingsOpen,
              }))
            }
          />
        }
        navRail={
          <ShellNavRail
            railId='story-shell-nav'
            open={chromeState.navOpen}
            onOpenChange={(open) =>
              setChromeState((current) => ({
                ...current,
                navOpen: open,
              }))
            }
            primaryItems={PRIMARY_ITEMS}
            activePrimarySection={chromeState.activePrimarySection}
            onSelectPrimarySection={(section) =>
              setChromeState((current) => ({
                ...current,
                activePrimarySection: section,
              }))
            }
            addTopicControl={
              <div className='composer composer-compact'>
                <label className='field flex flex-col gap-2'>
                  <span>Add Topic</span>
                  <input
                    className='h-11 w-full rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-input)] px-4 py-3 text-sm text-foreground'
                    placeholder='kukuri:topic:demo'
                    readOnly
                    value='kukuri:topic:design'
                  />
                </label>
                <Button variant='secondary'>Add</Button>
              </div>
            }
            topicList={
              <ul>
                <li className='topic-item topic-item-active'>
                  <button className='topic-link' type='button'>
                    <span className='shell-topic-link-label'>kukuri:topic:demo</span>
                  </button>
                  <div className='topic-diagnostic'>
                    <span>joined / peers: 1</span>
                    <small>12:45:11</small>
                  </div>
                </li>
                <li className='topic-item'>
                  <button className='topic-link' type='button'>
                    <span className='shell-topic-link-label'>kukuri:topic:staging-preview</span>
                  </button>
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>relay-assisted / peers: 1</span>
                    <small>no events</small>
                  </div>
                </li>
              </ul>
            }
            topicCount={2}
          />
        }
        workspace={
          <div className='shell-main-stack'>
            <Card className='shell-workspace-card'>
              <section className='shell-section' tabIndex={-1}>
                <div className='shell-workspace-header'>
                  <div>
                    <h2>Timeline</h2>
                    <span className='active-topic-label'>kukuri:topic:demo</span>
                  </div>
                  <div className='shell-inline-actions'>
                    <StatusBadge label='viewing public' />
                    <StatusBadge label='posting public' />
                    <Button
                      className='shell-context-trigger'
                      variant='ghost'
                      onClick={() =>
                        setChromeState((current) => ({
                          ...current,
                          contextOpen: true,
                        }))
                      }
                    >
                      Open Context
                    </Button>
                    <Button variant='secondary'>Refresh</Button>
                  </div>
                </div>
                <Notice tone='accent'>
                  Timeline header, scope selectors, and refresh remain in the primary workspace.
                </Notice>
              </section>

              <section className='shell-section' tabIndex={-1}>
                <Card className='panel-subsection'>
                  <h3>Channels & Composer</h3>
                  <p className='lede'>
                    Private channel controls, invite import, and the main composer stay together.
                  </p>
                </Card>
              </section>

              <section className='shell-section' tabIndex={-1}>
                <Card className='panel-subsection'>
                  <h3>Live</h3>
                  <p className='lede'>Live session entry remains in the workspace stack.</p>
                </Card>
              </section>

              <section className='shell-section' tabIndex={-1}>
                <Card className='panel-subsection'>
                  <h3>Game</h3>
                  <p className='lede'>Game room entry and score editing live below live sessions.</p>
                </Card>
              </section>

              <section className='shell-section' tabIndex={-1}>
                <Card className='panel-subsection'>
                  <h3>Profile</h3>
                  <p className='lede'>Profile editing is promoted into the primary workspace in Phase 3.</p>
                </Card>
              </section>

              <Card className='panel-subsection'>
                <h3>Timeline Feed</h3>
                <p className='lede'>Posts render last so composer-heavy flows stay above the fold.</p>
              </Card>
            </Card>
          </div>
        }
        contextPane={
          <ContextPane
            paneId='story-shell-context'
            open={chromeState.contextOpen}
            onOpenChange={(open) =>
              setChromeState((current) => ({
                ...current,
                contextOpen: open,
              }))
            }
            activeMode={chromeState.activeContextPaneMode}
            onModeChange={(mode) =>
              setChromeState((current) => ({
                ...current,
                activeContextPaneMode: mode,
              }))
            }
            tabs={contextTabs}
          />
        }
      />

      <SettingsDrawer
        drawerId='story-shell-settings'
        open={chromeState.settingsOpen}
        onOpenChange={(open) =>
          setChromeState((current) => ({
            ...current,
            settingsOpen: open,
          }))
        }
        activeSection={chromeState.activeSettingsSection}
        onSectionChange={(section) =>
          setChromeState((current) => ({
            ...current,
            activeSettingsSection: section,
          }))
        }
        sections={settingsSections}
      />
    </div>
  );
}

export const Wide: Story = {
  render: () => <ShellStoryFixture width={1360} />,
};

export const Narrow: Story = {
  render: () => <ShellStoryFixture width={700} />,
};

export const SettingsDrawerOpen: Story = {
  render: () => (
    <ShellStoryFixture
      width={1360}
      initialChromeState={{
        settingsOpen: true,
        activeSettingsSection: 'community-node',
      }}
    />
  ),
};

export const ContextThread: Story = {
  render: () => (
    <ShellStoryFixture
      width={1360}
      initialChromeState={{
        contextOpen: true,
        activeContextPaneMode: 'thread',
      }}
    />
  ),
};

export const ContextAuthor: Story = {
  render: () => (
    <ShellStoryFixture
      width={1360}
      initialChromeState={{
        contextOpen: true,
        activeContextPaneMode: 'author',
      }}
    />
  ),
};
