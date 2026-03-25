import { useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { ConnectivityPanel } from './ConnectivityPanel';
import { connectivityPanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const meta = {
  title: 'Settings/ConnectivityPanel',
  component: ConnectivityPanel,
  render: (args) => {
    const [peerTicketInput, setPeerTicketInput] = useState(args.view.peerTicketInput);

    return (
      <SettingsStoryFrame>
        <div>
          <ConnectivityPanel
            {...args}
            view={{ ...args.view, peerTicketInput }}
            onPeerTicketInputChange={setPeerTicketInput}
            onImportPeer={() => {}}
          />
        </div>
      </SettingsStoryFrame>
    );
  },
  args: {
    view: connectivityPanelFixture,
    onPeerTicketInputChange: () => {},
    onImportPeer: () => {},
  },
} satisfies Meta<typeof ConnectivityPanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const NarrowError: Story = {
  args: {
    view: {
      ...connectivityPanelFixture,
      status: 'error',
      summaryLabel: 'error',
      panelError: 'failed to import peer ticket: invalid endpoint id',
    },
  },
  render: (args) => {
    const [peerTicketInput, setPeerTicketInput] = useState(args.view.peerTicketInput);

    return (
      <SettingsStoryFrame width='narrow'>
        <div>
          <ConnectivityPanel
            {...args}
            view={{ ...args.view, peerTicketInput }}
            onPeerTicketInputChange={setPeerTicketInput}
            onImportPeer={() => {}}
          />
        </div>
      </SettingsStoryFrame>
    );
  },
};

export const Loading: Story = {
  args: {
    view: {
      ...connectivityPanelFixture,
      status: 'loading',
      summaryLabel: 'loading',
    },
  },
};
