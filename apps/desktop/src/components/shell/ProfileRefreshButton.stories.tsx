import type { Meta, StoryObj } from '@storybook/react-vite';
import { ProfileRefreshButton } from './ProfileRefreshButton';

const meta = {
  title: 'Shell/ProfileRefreshButton',
  component: ProfileRefreshButton,
  args: { refreshing: false, saving: false, onRefresh: async () => undefined },
} satisfies Meta<typeof ProfileRefreshButton>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Idle: Story = {};
export const Refreshing: Story = { args: { refreshing: true } };
export const Saving: Story = { args: { saving: true } };
