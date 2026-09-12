import type { Meta, StoryObj } from '@storybook/react-vite';
import { ProfileOverviewPanel } from './ProfileOverviewPanel';

const meta = {
  title: 'Extended/ProfileOverviewPanel',
  component: ProfileOverviewPanel,
  args: {
    authorLabel: 'きんぎょ', username: 'kingyosun', about: '小さな話題を持ち寄っています。',
    picture: null, status: 'ready', error: null, postCount: 1,
    followingCount: 3, followedCount: 2, mutedCount: 0, blockingCount: 0,
    onEdit: () => undefined, onOpenFollowing: () => undefined, onOpenFollowed: () => undefined,
    onOpenMuted: () => undefined, onOpenBlocking: () => undefined,
  },
  decorators: [(Story) => <div className='shell-phase1' style={{ maxWidth: 400 }}><Story /></div>],
} satisfies Meta<typeof ProfileOverviewPanel>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Ready: Story = {};
export const Empty: Story = { args: { postCount: 0, about: null } };
export const InitialLoading: Story = { args: { status: 'loading', postCount: null } };
export const RefreshError: Story = { args: { status: 'error', error: 'プロフィールを更新できませんでした。' } };
export const MissingUsername: Story = { args: { username: null } };
export const LongNames: Story = {
  args: { authorLabel: '長い表示名と絵文字🐟'.repeat(5), username: 'long_username_'.repeat(8) },
};
