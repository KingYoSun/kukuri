import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { DiscoveryPanel } from './DiscoveryPanel';
import { discoveryPanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type DiscoveryStoryProps = {
  args: ComponentProps<typeof DiscoveryPanel>;
  width?: 'wide' | 'narrow';
};

function DiscoveryPanelStory({
  args,
  width = 'wide',
}: DiscoveryStoryProps) {
  const [seedPeersInput, setSeedPeersInput] = useState(args.view.seedPeersInput);

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <DiscoveryPanel
          {...args}
          view={{ ...args.view, seedPeersInput }}
          onSeedPeersChange={setSeedPeersInput}
          onSave={() => {}}
          onReset={() => setSeedPeersInput(args.view.seedPeersInput)}
        />
      </div>
    </SettingsStoryFrame>
  );
}

const meta = {
  title: 'Settings/DiscoveryPanel',
  component: DiscoveryPanel,
  render: (args) => <DiscoveryPanelStory args={args} />,
  args: {
    view: discoveryPanelFixture,
    saveDisabled: false,
    resetDisabled: false,
    onSeedPeersChange: () => {},
    onSave: () => {},
    onReset: () => {},
  },
} satisfies Meta<typeof DiscoveryPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const NarrowLocked: Story = {
  args: {
    view: {
      ...discoveryPanelFixture,
      envLocked: true,
      seedPeersMessage: 'Environment overrides discovery seeds; editing is disabled.',
    },
    saveDisabled: true,
    resetDisabled: true,
  },
  render: (args) => <DiscoveryPanelStory args={args} width='narrow' />,
};

export const Loading: Story = {
  args: {
    view: {
      ...discoveryPanelFixture,
      status: 'loading',
      summaryLabel: 'loading',
    },
  },
};
