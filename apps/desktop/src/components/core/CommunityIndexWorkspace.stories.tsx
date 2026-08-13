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
    onSelectNode: () => {},
    manifests: {
      [node]: {
        status: 'ok',
        manifest: {
          node_id: 'community-example',
          node_name: 'Community Example',
          node_role: 'community-node',
          server_name: 'community.example',
          manifest_version: 'v1',
          capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
          authority_scope: { applies_to: ['this_node'], does_not_apply_to: [] },
          p2p_boundary: {
            identity_authority: false,
            profile_canonical_store: false,
            social_graph_canonical_store: false,
            content_truth_source: false,
            network_wide_authority: false,
          },
          abuse_contact: 'abuse@community.example',
          report_endpoint: `${node}/v1/report`,
          terms_url: '',
          privacy_url: '',
          moderation_policy_url: '',
        },
      },
    },
    onOpenCommunityNodeSettings: () => {},
  },
} satisfies Meta<typeof CommunityIndexWorkspace>;

export default meta;
type Story = StoryObj<typeof meta>;
export const Explore: Story = {};
export const Topic: Story = { args: { mode: 'topic' } };
export const Disabled: Story = {
  args: { eligibleNodeBaseUrls: [], selectedNodeBaseUrl: null, manifests: {} },
};
