import { useState, type ComponentProps } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { ConnectivityPanel } from './ConnectivityPanel';
import { connectivityPanelFixture } from './fixtures';
import { SettingsStoryFrame } from './SettingsStoryFrame';

type ConnectivityStoryProps = {
  args: ComponentProps<typeof ConnectivityPanel>;
  width?: 'wide' | 'narrow';
};

function ConnectivityPanelStory({
  args,
  width = 'wide',
}: ConnectivityStoryProps) {
  const [peerTicketInput, setPeerTicketInput] = useState(args.view.peerTicketInput);

  return (
    <SettingsStoryFrame width={width}>
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
}

const meta = {
  title: 'Settings/ConnectivityPanel',
  component: ConnectivityPanel,
  render: (args) => <ConnectivityPanelStory args={args} />,
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
  render: (args) => <ConnectivityPanelStory args={args} width='narrow' />,
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
