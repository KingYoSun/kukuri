import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { DiscoveryPanel } from './DiscoveryPanel';
import { discoveryPanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/DiscoveryPanel',
  component: DiscoveryPanel,
  render: (args) => {
    const [seedPeersInput, setSeedPeersInput] = useState(args.view.seedPeersInput);

    return (
      <SettingsStoryFrame>
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
  },
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
  render: (args) => (
    <SettingsStoryFrame width='narrow'>
      <div>
        <DiscoveryPanel {...args} onSeedPeersChange={() => {}} onSave={() => {}} onReset={() => {}} />
      </div>
    </SettingsStoryFrame>
  ),
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
