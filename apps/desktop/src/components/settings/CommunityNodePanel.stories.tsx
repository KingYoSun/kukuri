import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';
import { userEvent, within } from 'storybook/test';

import { CommunityNodePanel } from './CommunityNodePanel';
import { communityNodePanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type CommunityNodeStoryProps = {
  args: ComponentProps<typeof CommunityNodePanel>;
  width?: 'wide' | 'narrow';
};

function CommunityNodePanelStory({
  args,
  width = 'wide',
}: CommunityNodeStoryProps) {
  const [nodes, setNodes] = useState(args.view.nodes);
  const [indexNodePreference, setIndexNodePreference] = useState(
    args.indexNodePreference ?? { mode: 'auto' as const }
  );

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <CommunityNodePanel
          {...args}
          view={{ ...args.view, nodes }}
          indexNodePreference={indexNodePreference}
          onIndexNodePreferenceChange={setIndexNodePreference}
          onAddNode={() =>
            setNodes((current) => [
              ...current,
              {
                id: `story-node-${current.length + 1}`,
                baseUrl: '',
                saved: false,
                diagnostics: [],
                dependency: { diagnostics: [], boundaryNotes: [] },
                consent: {
                  loaded: false,
                  loading: false,
                  loadError: null,
                  withdrawn: false,
                  hasLocalConsent: false,
                  allRequiredAccepted: false,
                  hasPendingUpdate: false,
                  policies: [],
                },
                distanceOptoutEligible: true,
                inviteCodeSaved: false,
                admissionRejectionCode: null,
                lastError: null,
              },
            ])
          }
          onNodeBaseUrlChange={(id, value) =>
            setNodes((current) =>
              current.map((node) => (node.id === id ? { ...node, baseUrl: value } : node))
            )
          }
          onRemoveNode={(id) =>
            setNodes((current) => current.filter((node) => node.id !== id))
          }
          onSaveNodes={() => {}}
          onReset={() => setNodes(args.view.nodes)}
          onClearNodes={() => setNodes([])}
          onAuthenticate={() => {}}
          onFetchConsents={() => {}}
          onAcceptConsents={() => {}}
          onWithdrawConsents={() => {}}
          onRefresh={() => {}}
          onClearToken={() => {}}
          onSubmitInviteCode={args.onSubmitInviteCode}
        />
      </div>
    </SettingsStoryFrame>
  );
}

const meta = {
  title: 'Settings/CommunityNodePanel',
  component: CommunityNodePanel,
  render: (args) => <CommunityNodePanelStory args={args} />,
  args: {
    view: communityNodePanelFixture,
    saveDisabled: false,
    resetDisabled: false,
    clearDisabled: false,
    onAddNode: () => {},
    onNodeBaseUrlChange: () => {},
    onRemoveNode: () => {},
    onSaveNodes: () => {},
    onReset: () => {},
    onClearNodes: () => {},
    onAuthenticate: () => {},
    onFetchConsents: () => {},
    onAcceptConsents: () => {},
    onRefresh: () => {},
    onClearToken: () => {},
    onSubmitInviteCode: async () => {},
    indexNodePreference: { mode: 'auto' },
    eligibleIndexNodeBaseUrls: communityNodePanelFixture.nodes.map((node) => node.baseUrl),
    onGetRelationOptout: async () => ({
      pubkey: 'story-user',
      opted_out: false,
      opted_out_at: null,
      min_proximity: 0.25,
    }),
    onSetRelationOptout: async () => ({
      pubkey: 'story-user',
      opted_out: true,
      opted_out_at: new Date(0).toISOString(),
      min_proximity: 0.25,
    }),
    onClearRelationOptout: async () => ({
      pubkey: 'story-user',
      opted_out: false,
      opted_out_at: null,
      min_proximity: 0.25,
    }),
  },
} satisfies Meta<typeof CommunityNodePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const ManualIndexNode: Story = {
  args: {
    indexNodePreference: {
      mode: 'manual',
      baseUrl: communityNodePanelFixture.nodes[1].baseUrl,
    },
  },
};

export const ManualIndexNodeUnavailable: Story = {
  args: {
    indexNodePreference: {
      mode: 'manual',
      baseUrl: communityNodePanelFixture.nodes[1].baseUrl,
    },
    eligibleIndexNodeBaseUrls: [communityNodePanelFixture.nodes[0].baseUrl],
  },
};

export const DistanceOptout: Story = {
  args: { showDiagnostics: false },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getAllByRole('button', { name: 'Load setting' })[0]);
    canvasElement.ownerDocument.defaultView?.scrollTo(0, 0);
  },
};

export const InviteRequired: Story = {
  args: {
    showDiagnostics: false,
    view: {
      ...communityNodePanelFixture,
      nodes: communityNodePanelFixture.nodes.map((node, index) =>
        index === 0
          ? {
              ...node,
              distanceOptoutEligible: true,
              inviteCodeSaved: false,
              admissionRejectionCode: 'INVITE_REQUIRED' as const,
            }
          : node
      ),
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.type(canvas.getAllByLabelText(/Invite code|招待コード/)[0], 'join-code');
  },
};

export const NarrowError: Story = {
  args: {
    view: {
      ...communityNodePanelFixture,
      panelError: 'failed to update community nodes',
      editorMessage: 'Fix the invalid base URL before saving.',
      editorMessageTone: 'danger',
    },
  },
  render: (args) => <CommunityNodePanelStory args={args} width='narrow' />,
};

export const Loading: Story = {
  args: {
    view: {
      ...communityNodePanelFixture,
      status: 'loading',
      summaryLabel: 'loading',
    },
  },
};
