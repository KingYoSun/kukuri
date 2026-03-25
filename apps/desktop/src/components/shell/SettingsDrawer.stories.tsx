import { useMemo, useState } from 'react';

import type { Meta, StoryObj } from '@storybook/react-vite';

import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import {
  communityNodePanelFixture,
  connectivityPanelFixture,
  discoveryPanelFixture,
} from '@/components/settings/fixtures';
import type { SettingsSection } from '@/components/shell/types';

import { SettingsDrawer } from './SettingsDrawer';

function SettingsDrawerStory({ initialSection = 'connectivity' }: { initialSection?: SettingsSection }) {
  const [open, setOpen] = useState(true);
  const [activeSection, setActiveSection] = useState<SettingsSection>(initialSection);
  const [peerTicketInput, setPeerTicketInput] = useState(connectivityPanelFixture.peerTicketInput);
  const [seedPeersInput, setSeedPeersInput] = useState(discoveryPanelFixture.seedPeersInput);
  const [baseUrlsInput, setBaseUrlsInput] = useState(communityNodePanelFixture.baseUrlsInput);

  const sections = useMemo(
    () => [
      {
        id: 'connectivity' as const,
        label: 'Connectivity',
        description: 'Peer status and topic diagnostics',
        content: (
          <ConnectivityPanel
            view={{ ...connectivityPanelFixture, peerTicketInput }}
            onPeerTicketInputChange={setPeerTicketInput}
            onImportPeer={() => undefined}
          />
        ),
      },
      {
        id: 'discovery' as const,
        label: 'Discovery',
        description: 'Seeded DHT configuration and diagnostics',
        content: (
          <DiscoveryPanel
            view={{ ...discoveryPanelFixture, seedPeersInput }}
            saveDisabled={false}
            resetDisabled={false}
            onSeedPeersChange={setSeedPeersInput}
            onSave={() => undefined}
            onReset={() => setSeedPeersInput(discoveryPanelFixture.seedPeersInput)}
          />
        ),
      },
      {
        id: 'community-node' as const,
        label: 'Community Node',
        description: 'Auth, consent, and connectivity urls',
        content: (
          <CommunityNodePanel
            view={{ ...communityNodePanelFixture, baseUrlsInput }}
            saveDisabled={false}
            resetDisabled={false}
            clearDisabled={false}
            onBaseUrlsChange={setBaseUrlsInput}
            onSaveNodes={() => undefined}
            onReset={() => setBaseUrlsInput(communityNodePanelFixture.baseUrlsInput)}
            onClearNodes={() => setBaseUrlsInput('')}
            onAuthenticate={() => undefined}
            onFetchConsents={() => undefined}
            onAcceptConsents={() => undefined}
            onRefresh={() => undefined}
            onClearToken={() => undefined}
          />
        ),
      },
    ],
    [baseUrlsInput, peerTicketInput, seedPeersInput]
  );

  return (
    <SettingsDrawer
      drawerId='storybook-shell-settings'
      open={open}
      onOpenChange={setOpen}
      activeSection={activeSection}
      onSectionChange={setActiveSection}
      sections={sections}
    />
  );
}

const meta = {
  title: 'Shell/SettingsDrawer',
  component: SettingsDrawer,
  parameters: {
    layout: 'fullscreen',
  },
  args: {
    drawerId: 'storybook-shell-settings',
    open: true,
    onOpenChange: () => undefined,
    activeSection: 'connectivity',
    onSectionChange: () => undefined,
    sections: [],
  },
  render: () => <SettingsDrawerStory />,
} satisfies Meta<typeof SettingsDrawer>;

export default meta;

type Story = StoryObj<typeof meta>;

export const ConnectivityOpen: Story = {};

export const DiscoveryOpen: Story = {
  render: () => <SettingsDrawerStory initialSection='discovery' />,
};
