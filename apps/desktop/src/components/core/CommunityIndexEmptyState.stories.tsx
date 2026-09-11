import type { Meta, StoryObj } from '@storybook/react-vite';

import { CommunityIndexEmptyState } from './CommunityIndexEmptyState';
import { communityIndexEmptyGuidance } from './communityIndexEmptyGuidance';

const node = 'https://community-node-community-node-first.discovery.service.example.test';
const PUBKEY = 'a1'.repeat(32);

const meta = {
  title: 'Core/CommunityIndexEmptyState',
  component: CommunityIndexEmptyState,
  decorators: [
    (Story) => (
      <div className='shell-phase1' style={{ width: 'min(440px, 100vw)', padding: 0 }}>
        <div className='shell-column-body'><div className='shell-main-stack'><Story /></div></div>
      </div>
    ),
  ],
  args: {
    nodeBaseUrl: node,
    onRetry: () => {},
    onOpenAuthor: () => {},
    onOpenTimeline: () => {},
    onRequestIndexing: () => {},
    onOpenCommunityNodeSettings: () => {},
    onOpenConnectivitySettings: () => {},
    guidance: communityIndexEmptyGuidance({
      mode: 'explore',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
    }),
  },
} satisfies Meta<typeof CommunityIndexEmptyState>;

export default meta;
type Story = StoryObj<typeof meta>;

export const ExploreSearch: Story = {};

export const TopicSearch: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
    }),
  },
};

export const PrivateChannelSearch: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'design',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'channel', channel_id: 'channel-1' },
      activeChannelLabel: 'デザイン相談室',
      canRequestIndexing: true,
    }),
  },
};

export const UserIdQuery: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'explore',
      operation: 'search',
      query: PUBKEY,
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
    }),
  },
};

export const Discovery: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'explore',
      operation: 'discovery',
      query: '',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
    }),
  },
};

export const RetryPaused: Story = {
  args: { retryDisabled: true },
};

export const WithoutOptionalActions: Story = {
  args: {
    onOpenTimeline: undefined,
    onRequestIndexing: undefined,
    onOpenConnectivitySettings: undefined,
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: false,
    }),
  },
};
