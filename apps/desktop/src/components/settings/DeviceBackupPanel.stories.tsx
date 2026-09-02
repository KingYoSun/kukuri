import type { Meta, StoryObj } from '@storybook/react-vite';
import { userEvent, within } from 'storybook/test';

import { DeviceBackupPanel } from './DeviceBackupPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/DeviceBackupPanel',
  component: DeviceBackupPanel,
  render: () => (
    <SettingsStoryFrame width='narrow'>
      <DeviceBackupPanel />
    </SettingsStoryFrame>
  ),
} satisfies Meta<typeof DeviceBackupPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Initial: Story = {};

export const CreateReady: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByTestId('device-backup-acknowledge'));
    await userEvent.type(canvas.getByTestId('device-backup-passphrase'), 'long passphrase');
    await userEvent.type(canvas.getByTestId('device-backup-passphrase-confirm'), 'long passphrase');
  },
};

export const RestorePreview: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole('button', { name: /Choose backup file|ファイルを選択/ }));
    await userEvent.type(canvas.getByTestId('device-restore-passphrase'), 'long passphrase');
    await userEvent.click(canvas.getByRole('button', { name: /Review contents|内容を確認/ }));
  },
};
