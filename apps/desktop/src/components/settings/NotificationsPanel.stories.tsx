import type { Meta, StoryObj } from '@storybook/react-vite';
import { mocked } from 'storybook/test';

import {
  getOsNotificationPermission,
  requestOsNotificationPermission,
} from '@/lib/api/osNotificationPermission';
import { isTauriRuntime } from '@/lib/releaseReadiness';

import { NotificationsPanel } from './NotificationsPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

// #962: 通知の受信設定。Tauri の権限確認は available / unavailable / checking の 3 状態を固定する。
const meta = {
  title: 'Settings/NotificationsPanel',
  component: NotificationsPanel,
  render: (args) => (
    <SettingsStoryFrame width='wide'>
      <NotificationsPanel {...args} />
    </SettingsStoryFrame>
  ),
} satisfies Meta<typeof NotificationsPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

function notificationState(state: 'available' | 'unavailable' | 'checking') {
  mocked(isTauriRuntime).mockReturnValue(true);
  if (state === 'checking') {
    mocked(getOsNotificationPermission).mockReturnValue(new Promise(() => {}));
  } else {
    mocked(getOsNotificationPermission).mockResolvedValue(state);
  }
  mocked(requestOsNotificationPermission).mockResolvedValue('available');
  return () => {
    mocked(isTauriRuntime).mockRestore();
    mocked(getOsNotificationPermission).mockRestore();
    mocked(requestOsNotificationPermission).mockRestore();
  };
}

/** browser 既定: 権限は unknown、保存済み設定は既定値。 */
export const Browser: Story = {};

export const NotificationServiceAvailable: Story = {
  beforeEach: () => notificationState('available'),
};

export const NotificationServiceUnavailable: Story = {
  beforeEach: () => notificationState('unavailable'),
};

export const NotificationServiceChecking: Story = {
  beforeEach: () => notificationState('checking'),
};

export const Narrow: Story = {
  render: (args) => (
    <SettingsStoryFrame width='narrow'>
      <NotificationsPanel {...args} />
    </SettingsStoryFrame>
  ),
  beforeEach: () => notificationState('available'),
};
