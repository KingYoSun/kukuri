import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { CommunityIndexingRequestDialog } from './CommunityIndexingRequestDialog';

const meta = {
  title: 'Core/CommunityIndexingRequestDialog',
  component: CommunityIndexingRequestDialog,
  args: {
    api: createDesktopMockApi(),
    target: {
      kind: 'private_channel',
      topicId: 'kukuri:topic:demo',
      channelId: 'channel-core',
      channelLabel: 'Core Contributors',
    },
    eligibleNodeBaseUrls: ['https://community.example'],
    onOpenChange: () => undefined,
    onOpenCommunityNodeSettings: () => undefined,
  },
} satisfies Meta<typeof CommunityIndexingRequestDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const PrivateChannelConfirmation: Story = {};

export const NoEligibleNode: Story = {
  args: {
    target: { kind: 'public_topic', topicId: 'kukuri:topic:demo' },
    eligibleNodeBaseUrls: [],
  },
};
