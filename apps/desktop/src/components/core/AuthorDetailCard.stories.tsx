import type { Meta, StoryObj } from '@storybook/react-vite';
import { userEvent, within } from 'storybook/test';

import {
  createStoryAuthorDetailView,
  STORY_EMPTY_AUTHOR_DETAIL_VIEW,
} from '@/components/storyFixtures';

import { AuthorDetailCard } from './AuthorDetailCard';
import { CommunityNodeAdvisoryPanel } from './CommunityNodeAdvisoryPanel';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';

const authorDetailView = createStoryAuthorDetailView();
const advisoryApi = {
  ...createDesktopMockApi(),
  async readCommunityNodeTrustUser(request: { target_pubkey: string }) {
    return {
      viewer_pubkey: 'story-viewer',
      target_id: request.target_pubkey,
      absolute: 0.35,
      relative: 0.62,
      trust: 0.485,
      w_abs_applied: 0.5,
      computed_at: '2026-08-14T00:00:00Z',
      basis: [
        {
          signal_id: 'story-signal',
          issuer_node_id: 'community.example.com',
          component: 'relative' as const,
          category: 'spam' as const,
          severity: 'low' as const,
          basis: 'provider_verdict' as const,
          confidence: 0.78,
          visibility: 'subscribed_nodes' as const,
          appeal_status: 'none' as const,
          expires_at: null,
          raw_contribution: 0.2,
          decay_factor: 0.9,
          relation_weight: 1,
          contribution: 0.18,
        },
      ],
    };
  },
  async listCommunityNodeRelationNeighbors() {
    return { viewer_pubkey: 'story-viewer', neighbors: ['c'.repeat(64)] };
  },
};

const meta = {
  title: 'Core/AuthorDetailCard',
  component: AuthorDetailCard,
  parameters: { layout: 'padded' },
  render: (args) => (
    <div style={{ maxWidth: '420px' }}>
      <AuthorDetailCard {...args} />
    </div>
  ),
  args: {
    view: authorDetailView,
    localAuthorPubkey: 'f'.repeat(64),
    onToggleRelationship: () => undefined,
    onToggleMute: () => undefined,
  },
} satisfies Meta<typeof AuthorDetailCard>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Selected: Story = {};

export const CommunityNodeAdvisory: Story = {
  args: {
    communityNodeAdvisory: (
      <CommunityNodeAdvisoryPanel
        api={advisoryApi}
        targetPubkey={authorDetailView.author?.author_pubkey ?? 'a'.repeat(64)}
        nodeBaseUrls={['https://community.example.com']}
      />
    ),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole('button', { name: 'Load advisory' }));
    await userEvent.click(await canvas.findByText(/community\.example\.com · spam · low/));
    canvasElement.ownerDocument.defaultView?.scrollTo(0, 0);
  },
};

export const Empty: Story = {
  args: {
    view: STORY_EMPTY_AUTHOR_DETAIL_VIEW,
  },
};
