import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { DeveloperLogViewer } from './DeveloperLogViewer';
import { DeveloperPanel } from './DeveloperPanel';
import { developerLogFixtureSnapshot } from '@/mocks/api/developerLogs';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type DeveloperStoryProps = {
  args: ComponentProps<typeof DeveloperPanel>;
  width?: 'wide' | 'narrow';
};

// #978: 本番では shell の hook が取得する。story は固定 fixture を ON 時の slot に渡す。
const logsSnapshot = developerLogFixtureSnapshot();
const logsFixture = (
  <DeveloperLogViewer
    status='ready'
    view={{
      entries: logsSnapshot.entries,
      oldestSeq: logsSnapshot.oldest_seq,
      nextSeq: logsSnapshot.next_seq,
      maxEntries: logsSnapshot.max_entries,
      maxBytes: logsSnapshot.max_bytes,
      droppedOlder: false,
      gapSinceLastRefresh: false,
    }}
    onRefresh={() => {}}
  />
);

function DeveloperPanelStory({ args, width = 'wide' }: DeveloperStoryProps) {
  const [enabled, setEnabled] = useState(args.developerModeEnabled);

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <DeveloperPanel
          {...args}
          developerModeEnabled={enabled}
          onDeveloperModeChange={setEnabled}
          logs={enabled ? logsFixture : null}
        />
      </div>
    </SettingsStoryFrame>
  );
}

const meta = {
  title: 'Settings/DeveloperPanel',
  component: DeveloperPanel,
  render: (args) => <DeveloperPanelStory args={args} />,
  args: {
    developerModeEnabled: false,
    onDeveloperModeChange: () => {},
    onOpenDiagnostics: () => {},
  },
} satisfies Meta<typeof DeveloperPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Disabled: Story = {};

export const Enabled: Story = {
  args: {
    developerModeEnabled: true,
  },
};

export const Narrow: Story = {
  args: { developerModeEnabled: true },
  render: (args) => <DeveloperPanelStory args={args} width='narrow' />,
};
