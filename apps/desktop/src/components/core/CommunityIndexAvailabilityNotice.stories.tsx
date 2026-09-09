import type { Meta, StoryObj } from '@storybook/react-vite';
import { CommunityIndexAvailabilityNotice } from './CommunityIndexAvailabilityNotice';
import type { CommunityNodeAvailability } from '@/lib/api/communityNodeAvailability';

const availability: CommunityNodeAvailability = {
  reason: 'checking', recovery: 'status', baseUrl: 'https://api.kukuri.app', manual: false, retryAfter: null,
};
const meta = {
  title: 'Core/CommunityIndexAvailabilityNotice',
  component: CommunityIndexAvailabilityNotice,
  decorators: [(Story) => <div style={{ width: 'min(360px, 100vw)' }}><Story /></div>],
  args: { availability, onRetry: async () => {}, onReviewPolicies: () => {}, onOpenSettings: () => {}, onAutomatic: () => {} },
} satisfies Meta<typeof CommunityIndexAvailabilityNotice>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Checking: Story = {};
export const NoNodes: Story = { args: { availability: { ...availability, reason: 'noNodes', recovery: 'settings' } } };
export const ReadError: Story = { args: { availability: { ...availability, reason: 'statusUnavailable' } } };
export const ConsentRequired: Story = { args: { availability: { ...availability, reason: 'consentRequired', recovery: 'consent' } } };
export const Reconsent: Story = { args: { availability: { ...availability, reason: 'reconsentRequired', recovery: 'consent' } } };
export const Connecting: Story = { args: { availability: { ...availability, reason: 'connecting', recovery: null } } };
export const Retrying: Story = { args: { availability: { ...availability, reason: 'retrying', recovery: 'metadata' } } };
export const Invite: Story = { args: { availability: { ...availability, reason: 'inviteRequired', recovery: 'settings' } } };
export const AdmissionDenied: Story = { args: { availability: { ...availability, reason: 'admissionDenied', recovery: 'settings' } } };
export const ManifestError: Story = { args: { availability: { ...availability, reason: 'manifestError', recovery: 'manifest' } } };
export const ManifestAbsent: Story = { args: { availability: { ...availability, reason: 'manifestAbsent', recovery: 'manifest' } } };
export const SearchUnavailable: Story = { args: { availability: { ...availability, reason: 'indexNotProvided', recovery: 'settings' } } };
export const Authentication: Story = { args: { availability: { ...availability, reason: 'authRequired', recovery: 'metadata' } } };
export const ManualFailure: Story = { args: { availability: { ...availability, reason: 'connectionFailed', recovery: 'metadata', manual: true } } };
export const LongNodeAddress: Story = {
  args: {
    availability: {
      ...availability,
      reason: 'connectionFailed', recovery: 'metadata', manual: true,
      baseUrl: 'https://community-node-community-node-community-node-first.discovery.service.example.test',
    },
  },
};
