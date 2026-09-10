import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, userEvent, waitFor, within } from 'storybook/test';

import i18n from '@/i18n';
import { ReactionsPanel } from './ReactionsPanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/ReactionsPanel',
  component: ReactionsPanel,
  args: {
    view: { status: 'ready', summaryLabel: '', ownedAssets: [], bookmarkedAssets: [] },
    creating: false,
    onCreateAsset: () => undefined,
    onRemoveBookmark: async () => undefined,
  },
  render: args => <SettingsStoryFrame width='narrow'><ReactionsPanel {...args} /></SettingsStoryFrame>,
} satisfies Meta<typeof ReactionsPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {};
export const Saving: Story = { args: { creating: true } };
export const SelectedLongName: Story = {
  play: async ({ canvasElement }) => {
    const png = Uint8Array.from(atob('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a/A8AAAAASUVORK5CYII='), c => c.charCodeAt(0));
    const name = 'リアクション用の長いファイル名'.repeat(4) + '.png';
    const input = canvasElement.querySelector<HTMLInputElement>('input[type=file]')!;
    const selection = new DataTransfer();
    selection.items.add(new File([png], name, { type: 'image/png' }));
    input.files = selection.files;
    input.dispatchEvent(new Event('change', { bubbles: true }));
    const page = within(canvasElement.ownerDocument.body);
    const crop = await page.findByRole('dialog');
    const save = within(crop).getByRole('button', { name: i18n.t('common:actions.save') });
    await waitFor(() => expect(save).toBeEnabled());
    await userEvent.click(save);
    await waitFor(() => expect(crop).not.toBeInTheDocument());
    await expect(within(canvasElement).getByText(name)).toBeVisible();
  },
};
