import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { SafetyPanel } from './SafetyPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type SafetyStoryProps = {
  args: ComponentProps<typeof SafetyPanel>;
  width?: 'wide' | 'narrow';
};

function SafetyPanelStory({ args, width = 'wide' }: SafetyStoryProps) {
  const [enabled, setEnabled] = useState(args.adultContentEnabled);

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <SafetyPanel
          adultContentEnabled={enabled}
          onAdultContentEnabledChange={setEnabled}
          onOpenMutedUsers={args.onOpenMutedUsers}
          onOpenBlockedUsers={args.onOpenBlockedUsers}
        />
      </div>
    </SettingsStoryFrame>
  );
}

const meta = {
  title: 'Settings/SafetyPanel',
  component: SafetyPanel,
  render: (args) => <SafetyPanelStory args={args} />,
  args: {
    adultContentEnabled: false,
    onAdultContentEnabledChange: () => {},
    onOpenMutedUsers: () => {},
    onOpenBlockedUsers: () => {},
  },
} satisfies Meta<typeof SafetyPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Disabled: Story = {};

export const Enabled: Story = {
  args: {
    adultContentEnabled: true,
  },
};

export const Narrow: Story = {
  render: (args) => <SafetyPanelStory args={args} width='narrow' />,
};
