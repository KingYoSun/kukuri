import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { TesterFeedbackDialog } from './TesterFeedbackDialog';

const meta = {
  title: 'Core/TesterFeedbackDialog',
  component: TesterFeedbackDialog,
  args: {
    api: createDesktopMockApi(),
    open: true,
    eligibleNodeBaseUrls: ['https://community.example'],
    onOpenChange: () => undefined,
    onOpenCommunityNodeSettings: () => undefined,
  },
} satisfies Meta<typeof TesterFeedbackDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};

export const NoEligibleNode: Story = {
  args: {
    eligibleNodeBaseUrls: [],
  },
};
