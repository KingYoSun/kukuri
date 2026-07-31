import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { DeveloperPanel } from './DeveloperPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type DeveloperStoryProps = {
  args: ComponentProps<typeof DeveloperPanel>;
  width?: 'wide' | 'narrow';
};

function DeveloperPanelStory({ args, width = 'wide' }: DeveloperStoryProps) {
  const [enabled, setEnabled] = useState(args.developerModeEnabled);

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <DeveloperPanel developerModeEnabled={enabled} onDeveloperModeChange={setEnabled} />
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
  render: (args) => <DeveloperPanelStory args={args} width='narrow' />,
};
