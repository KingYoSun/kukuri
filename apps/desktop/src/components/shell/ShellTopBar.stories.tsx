import type { Meta, StoryObj } from '@storybook/react-vite';

import { StatusBadge } from '@/components/StatusBadge';
import { STORY_ACTIVE_TOPIC } from '@/components/storyFixtures';

import { ShellTopBar } from './ShellTopBar';

const statusBadges = (
  <>
    <StatusBadge label='connected' tone='accent' />
    <StatusBadge label='2 peers' />
    <StatusBadge label='seeded dht' />
  </>
);

const meta = {
  title: 'Shell/ShellTopBar',
  component: ShellTopBar,
  args: {
    headline: 'Seeded DHT + direct peers',
    activeTopic: STORY_ACTIVE_TOPIC,
    statusBadges,
    navOpen: false,
    settingsOpen: false,
    navControlsId: 'storybook-shell-nav',
    settingsControlsId: 'storybook-shell-settings',
    onToggleNav: () => undefined,
    onToggleSettings: () => undefined,
  },
} satisfies Meta<typeof ShellTopBar>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const PanelsOpen: Story = {
  args: {
    navOpen: true,
    settingsOpen: true,
  },
};
