import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { type DesktopTheme } from '@/lib/theme';

import { createAppearancePanelFixture } from './fixtures';
import { AppearancePanel } from './AppearancePanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const appearancePanelFixture = createAppearancePanelFixture();

const meta = {
  title: 'Settings/AppearancePanel',
  component: AppearancePanel,
  args: {
    view: appearancePanelFixture,
    onThemeChange: () => undefined,
    onLocaleChange: () => undefined,
  },
  render: () => <AppearancePanelStory />,
} satisfies Meta<typeof AppearancePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

function AppearancePanelStory() {
  const [theme, setTheme] = useState<DesktopTheme>(appearancePanelFixture.selectedTheme);
  const [locale, setLocale] = useState(appearancePanelFixture.selectedLocale);

  return (
    <SettingsStoryFrame width='narrow'>
      <AppearancePanel
        view={{ ...createAppearancePanelFixture(), selectedTheme: theme, selectedLocale: locale }}
        onThemeChange={setTheme}
        onLocaleChange={setLocale}
      />
    </SettingsStoryFrame>
  );
}

export const Default: Story = {};
