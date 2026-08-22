import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

const api = createDesktopMockApi();
const node = 'https://community.example';

const meta = {
  title: 'Core/CommunityIndexWorkspace',
  component: CommunityIndexWorkspace,
  args: {
    api,
    mode: 'explore',
    locale: 'en',
    activeTopic: 'kukuri:topic:demo',
    activeTimelineScope: { kind: 'public' },
    eligibleNodeBaseUrls: [node],
    selectedNodeBaseUrl: node,
    onOpenCommunityNodeSettings: () => {},
  },
} satisfies Meta<typeof CommunityIndexWorkspace>;

export default meta;
type Story = StoryObj<typeof meta>;
export const Explore: Story = {};
export const Topic: Story = { args: { mode: 'topic' } };
export const Disabled: Story = {
  args: { eligibleNodeBaseUrls: [], selectedNodeBaseUrl: null },
};
