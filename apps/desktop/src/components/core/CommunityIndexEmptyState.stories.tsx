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

// #975: 索引状況が確定した空状態。理由が断定文になり、申請済み・索引対象では申請 CTA が消える。
export const TopicNotIndexed: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
      indexStatus: {
        kind: 'known',
        response: {
          requests: [],
          target: { scope_kind: 'public_topic', scope_id: 'kukuri:topic:general', supported: false },
        },
      },
    }),
  },
};

export const TopicRequestPending: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
      indexStatus: {
        kind: 'known',
        response: {
          requests: [{
            request_id: 'request-1',
            scope_kind: 'public_topic',
            target_id: 'kukuri:topic:general',
            status: 'pending',
            created_at: 1_757_500_000_000,
            decided_at: null,
          }],
          target: { scope_kind: 'public_topic', scope_id: 'kukuri:topic:general', supported: false },
        },
      },
    }),
  },
};

export const ExploreOwnRequests: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'explore',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
      indexStatus: {
        kind: 'known',
        response: {
          requests: [
            { request_id: 'r1', scope_kind: 'public_topic', target_id: 'kukuri:topic:rust', status: 'approved', created_at: 1, decided_at: 2 },
            { request_id: 'r2', scope_kind: 'public_topic', target_id: 'kukuri:topic:golang', status: 'pending', created_at: 3, decided_at: null },
          ],
          target: null,
        },
      },
    }),
  },
};

export const StatusUnavailable: Story = {
  args: {
    guidance: communityIndexEmptyGuidance({
      mode: 'topic',
      operation: 'search',
      query: 'CliPeerA',
      activeTopic: 'kukuri:topic:general',
      activeTimelineScope: { kind: 'public' },
      canRequestIndexing: true,
      indexStatus: { kind: 'unknown' },
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
