import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { type DesktopTheme } from '@/lib/theme';
import { useTranslation } from 'react-i18next';
import { normalizeSupportedLocale } from '@/i18n';
import { changeDesktopLocale } from '@/i18n/changeLocale';

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
  render: (args) => <AppearancePanelStory saveFailed={args.localeSaveFailed} />,
} satisfies Meta<typeof AppearancePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

function AppearancePanelStory({ saveFailed }: { saveFailed?: boolean }) {
  const { i18n } = useTranslation();
  const [theme, setTheme] = useState<DesktopTheme>(appearancePanelFixture.selectedTheme);
  const locale = normalizeSupportedLocale(i18n.resolvedLanguage);

  return (
    <SettingsStoryFrame width='narrow'>
      <AppearancePanel
        view={{ ...createAppearancePanelFixture(), selectedTheme: theme, selectedLocale: locale }}
        onThemeChange={setTheme}
        onLocaleChange={changeDesktopLocale}
        localeSaveFailed={saveFailed}
      />
    </SettingsStoryFrame>
  );
}

export const Default: Story = {};
export const LocaleSaveError: Story = { args: { localeSaveFailed:true } };
