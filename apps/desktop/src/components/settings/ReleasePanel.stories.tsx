import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';

import { ReleasePanel } from './ReleasePanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const store = createDesktopShellStore();

const meta = {
  title: 'Settings/ReleasePanel',
  component: ReleasePanel,
  render: (args) => (
    <DesktopShellStoreContext.Provider value={store}>
      <SettingsStoryFrame width='wide'>
        <ReleasePanel {...args} />
      </SettingsStoryFrame>
    </DesktopShellStoreContext.Provider>
  ),
  args: {
    showDiagnostics: true,
  },
} satisfies Meta<typeof ReleasePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Resources: Story = {};
