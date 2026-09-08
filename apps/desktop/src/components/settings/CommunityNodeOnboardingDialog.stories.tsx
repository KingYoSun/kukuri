import type { Meta, StoryObj } from '@storybook/react-vite';
import { CommunityNodeOnboardingDialog } from './CommunityNodeOnboardingDialog';

const meta = {
  title: 'Settings/CommunityNodeOnboardingDialog',
  component: CommunityNodeOnboardingDialog,
  args: {
    baseUrl: 'https://api.kukuri.app', nodeLabel: 'kukuri.app',
    onDismiss: () => {}, onReview: () => {}, onOpenSettings: () => {},
  },
} satisfies Meta<typeof CommunityNodeOnboardingDialog>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Introduction: Story = {};
export const CustomNode: Story = {
  args: {
    baseUrl: 'https://community.long-name.example/our-community',
    nodeLabel: '地域の参加者が運営するコミュニティノード / Community for local members',
  },
};
