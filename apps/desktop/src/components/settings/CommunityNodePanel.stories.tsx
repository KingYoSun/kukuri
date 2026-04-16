import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

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

  return (
    <SettingsStoryFrame width={width}>
      <div>
        <CommunityNodePanel
          {...args}
          view={{ ...args.view, nodes }}
          onAddNode={() =>
            setNodes((current) => [
              ...current,
              {
                id: `story-node-${current.length + 1}`,
                baseUrl: '',
                autoApprove: false,
                saved: false,
                diagnostics: [],
                lastError: null,
              },
            ])
          }
          onNodeBaseUrlChange={(id, value) =>
            setNodes((current) =>
              current.map((node) => (node.id === id ? { ...node, baseUrl: value } : node))
            )
          }
          onNodeAutoApproveChange={(id, value) =>
            setNodes((current) =>
              current.map((node) => (node.id === id ? { ...node, autoApprove: value } : node))
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
          onRefresh={() => {}}
          onClearToken={() => {}}
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
    onNodeAutoApproveChange: () => {},
    onRemoveNode: () => {},
    onSaveNodes: () => {},
    onReset: () => {},
    onClearNodes: () => {},
    onAuthenticate: () => {},
    onFetchConsents: () => {},
    onAcceptConsents: () => {},
    onRefresh: () => {},
    onClearToken: () => {},
  },
} satisfies Meta<typeof CommunityNodePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

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
