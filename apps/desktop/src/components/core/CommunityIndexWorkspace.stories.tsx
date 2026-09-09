import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

const api = createDesktopMockApi();
const node = 'https://community.example';

const meta = {
  title: 'Core/CommunityIndexWorkspace',
  component: CommunityIndexWorkspace,
  decorators: [
    (Story) => (
      <div className='shell-phase1' style={{ width: 'min(440px, 100vw)', padding: 0 }}>
        <div className='shell-column-body'><div className='shell-main-stack'><Story /></div></div>
      </div>
    ),
  ],
  args: {
    api,
    mode: 'explore',
    activeTopic: 'kukuri:topic:demo',
    activeTimelineScope: { kind: 'public' },
    eligibleNodeBaseUrls: [node],
    selectedNodeBaseUrl: node,
    onOpenCommunityNodeSettings: () => {},
    onOpenAuthor: () => {},
  },
} satisfies Meta<typeof CommunityIndexWorkspace>;

export default meta;
type Story = StoryObj<typeof meta>;
export const Explore: Story = {};
export const Topic: Story = { args: { mode: 'topic' } };
export const Disabled: Story = {
  args: { eligibleNodeBaseUrls: [], selectedNodeBaseUrl: null },
};
export const LongPolicyLabels: Story = {
  args: {
    consentPendingNodeBaseUrls: [
      'https://community-node-community-node-community-node-first.discovery.service.example.test',
      'https://community-node-community-node-community-node-second.discovery.service.example.test',
    ],
  },
};
