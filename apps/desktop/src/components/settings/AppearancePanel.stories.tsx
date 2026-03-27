import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { type DesktopTheme } from '@/lib/theme';

import { appearancePanelFixture } from './fixtures';
import { AppearancePanel } from './AppearancePanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/AppearancePanel',
  component: AppearancePanel,
  args: {
    view: appearancePanelFixture,
    onThemeChange: () => undefined,
  },
  render: () => <AppearancePanelStory />,
} satisfies Meta<typeof AppearancePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

function AppearancePanelStory() {
  const [theme, setTheme] = useState<DesktopTheme>(appearancePanelFixture.selectedTheme);

  return (
    <SettingsStoryFrame width='narrow'>
      <AppearancePanel
        view={{ ...appearancePanelFixture, selectedTheme: theme }}
        onThemeChange={setTheme}
      />
    </SettingsStoryFrame>
  );
}

export const Default: Story = {};
