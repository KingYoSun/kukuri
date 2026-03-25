import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { CommunityNodePanel } from './CommunityNodePanel';
import { communityNodePanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/CommunityNodePanel',
  component: CommunityNodePanel,
  render: (args) => {
    const [baseUrlsInput, setBaseUrlsInput] = useState(args.view.baseUrlsInput);

    return (
      <SettingsStoryFrame>
        <div>
          <CommunityNodePanel
            {...args}
            view={{ ...args.view, baseUrlsInput }}
            onBaseUrlsChange={setBaseUrlsInput}
            onSaveNodes={() => {}}
            onReset={() => setBaseUrlsInput(args.view.baseUrlsInput)}
            onClearNodes={() => setBaseUrlsInput('')}
            onAuthenticate={() => {}}
            onFetchConsents={() => {}}
            onAcceptConsents={() => {}}
            onRefresh={() => {}}
            onClearToken={() => {}}
          />
        </div>
      </SettingsStoryFrame>
    );
  },
  args: {
    view: communityNodePanelFixture,
    saveDisabled: false,
    resetDisabled: false,
    clearDisabled: false,
    onBaseUrlsChange: () => {},
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
  render: (args) => (
    <SettingsStoryFrame width='narrow'>
      <div>
        <CommunityNodePanel
          {...args}
          onBaseUrlsChange={() => {}}
          onSaveNodes={() => {}}
          onReset={() => {}}
          onClearNodes={() => {}}
          onAuthenticate={() => {}}
          onFetchConsents={() => {}}
          onAcceptConsents={() => {}}
          onRefresh={() => {}}
          onClearToken={() => {}}
        />
      </div>
    </SettingsStoryFrame>
  ),
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
