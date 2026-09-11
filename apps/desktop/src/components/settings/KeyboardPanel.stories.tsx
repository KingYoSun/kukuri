import type { Meta, StoryObj } from '@storybook/react-vite';

import { KeyboardPanel } from './KeyboardPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/KeyboardPanel',
  component: KeyboardPanel,
  render: () => (
    <SettingsStoryFrame>
      <div>
        <KeyboardPanel />
      </div>
    </SettingsStoryFrame>
  ),
} satisfies Meta<typeof KeyboardPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const Narrow: Story = {
  render: () => (
    <SettingsStoryFrame width='narrow'>
      <div>
        <KeyboardPanel />
      </div>
    </SettingsStoryFrame>
  ),
};
