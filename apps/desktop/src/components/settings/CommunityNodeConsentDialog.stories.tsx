import type { Meta, StoryObj } from '@storybook/react-vite';
import { CommunityNodeConsentDialog } from './CommunityNodeConsentDialog';
import { communityNodePanelFixture } from './fixtures';

const consent = {
  ...communityNodePanelFixture.nodes[0].consent,
  allRequiredAccepted: false, hasLocalConsent: false,
  policies: communityNodePanelFixture.nodes[0].consent.policies.map((policy) => ({
    ...policy, acceptedAtLabel: null,
  })),
};
const meta = {
  title: 'Settings/CommunityNodeConsentDialog',
  component: CommunityNodeConsentDialog,
  args: {
    open: true, baseUrl: 'https://api.kukuri.app', consent, busy: false,
    onOpenChange: () => {}, onAccept: () => {}, onRetry: () => {},
  },
} satisfies Meta<typeof CommunityNodeConsentDialog>;
export default meta;
type Story = StoryObj<typeof meta>;
export const Review: Story = {};
export const Loading: Story = { args: { consent: { ...consent, loaded: false, loading: true, policies: [] } } };
export const LoadError: Story = { args: { consent: { ...consent, loaded: false, loadError: 'The connection is unavailable.', policies: [] } } };
export const Empty: Story = { args: { consent: { ...consent, policies: [] } } };
export const Accepting: Story = { args: { busy: true } };
export const AcceptError: Story = { args: { error: 'Consent could not be completed. Check the displayed terms and try again.' } };
export const Accepted: Story = { args: { consent: communityNodePanelFixture.nodes[0].consent } };
export const Updated: Story = { args: { consent: { ...consent, hasLocalConsent: true, hasPendingUpdate: true,
  policies: consent.policies.map((policy) => ({ ...policy, updated: true, policyVersion: 2, previouslyAcceptedVersion: 1 })),
} } };
export const Withdrawn: Story = { args: { consent: { ...consent, withdrawn: true } } };
