import type { Meta, StoryObj } from '@storybook/react-vite';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';

import { TesterFeedbackDialog } from './TesterFeedbackDialog';
import type { TesterFeedbackNodeReason } from '@/lib/api/testerFeedbackAvailability';

const meta = {
  title: 'Core/TesterFeedbackDialog',
  component: TesterFeedbackDialog,
  args: {
    api: createDesktopMockApi(),
    open: true,
    eligibleNodeBaseUrls: ['https://community.example'],
    availability: { state: 'ready', nodes: [] },
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
    availability: { state: 'noNodes', nodes: [] },
  },
};

export const Checking: Story = { args: { eligibleNodeBaseUrls: [], availability: { state: 'checking', nodes: [] } } };
export const ConfigUnavailable: Story = { args: { eligibleNodeBaseUrls: [], availability: { state: 'configUnavailable', nodes: [] } } };
export const StatusUnavailable: Story = { args: { eligibleNodeBaseUrls: [], availability: { state: 'statusUnavailable', nodes: [] } } };
export const CachedStatusUnavailable: Story = { args: { availability: { state: 'statusUnavailable', nodes: [] } } };

function nodeStory(reason: TesterFeedbackNodeReason): Story {
  return { args: { eligibleNodeBaseUrls: [], availability: { state: 'unavailable', nodes: [
    { baseUrl: 'https://community.example', label: 'Community for builders / 開発者コミュニティ / 开发者社区', reason },
  ] } } };
}

export const NotProvided: Story = nodeStory('notProvided');
export const ConsentRequired: Story = nodeStory('consentRequired');
export const ReconsentRequired: Story = nodeStory('reconsentRequired');
export const AuthRequired: Story = nodeStory('authRequired');
export const Connecting: Story = nodeStory('connecting');
export const Retrying: Story = nodeStory('retrying');
export const ConnectionFailed: Story = nodeStory('connectionFailed');
export const AdmissionRequired: Story = nodeStory('admissionRequired');
export const ManifestUnavailable: Story = nodeStory('manifestUnavailable');
export const NodeChecking: Story = nodeStory('checking');
export const MixedUnavailable: Story = { args: { eligibleNodeBaseUrls: [], availability: {
  state: 'unavailable', nodes: [
    { baseUrl: 'https://search.example', label: 'Search community', reason: 'notProvided' },
    { baseUrl: 'https://feedback.example', label: 'Feedback community', reason: 'consentRequired' },
  ],
} } };
