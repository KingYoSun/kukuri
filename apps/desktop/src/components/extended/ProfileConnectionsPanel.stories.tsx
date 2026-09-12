import type { Meta, StoryObj } from '@storybook/react-vite';
import { ProfileConnectionsPanel } from './ProfileConnectionsPanel';

const author = {
  author_pubkey: 'b'.repeat(64), name: 'browser peer', display_name: null, about: null,
  picture_asset: null, following: true, followed_by: true, mutual: true,
  friend_of_friend: false, friend_of_friend_via_pubkeys: [], muted: true,
  blocking: true, blocked_by: false, provenance: null,
};
const meta = {
  title: 'Extended/ProfileConnectionsPanel', component: ProfileConnectionsPanel,
  args: {
    activeView: 'blocking', items: [author], localAuthorPubkey: 'a'.repeat(64),
    status: 'ready', error: null, onSelectView: () => undefined,
    onToggleRelationship: () => undefined, onToggleMute: () => undefined,
    onToggleBlock: () => undefined, onBack: () => undefined,
  },
  decorators: [(Story) => <div className='shell-phase1' style={{ maxWidth: 400 }}><Story /></div>],
} satisfies Meta<typeof ProfileConnectionsPanel>;
export default meta;
type Story = StoryObj<typeof meta>;

export const Blocked: Story = {};
export const Following: Story = { args: { activeView: 'following' } };
export const Followers: Story = { args: { activeView: 'followed' } };
export const Muted: Story = { args: { activeView: 'muted' } };
export const Empty: Story = { args: { items: [] } };
export const Loading: Story = { args: { items: [], status: 'loading' } };
export const Error: Story = { args: { status: 'error', error: '一覧を更新できませんでした。' } };
export const Self: Story = { args: { localAuthorPubkey: author.author_pubkey } };
export const LongContent: Story = {
  args: { items: [{ ...author, display_name: '長い名前🐟'.repeat(12), about: '長い自己紹介とURL https://example.test/'.repeat(8) }] },
};
