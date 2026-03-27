import type {
  AppearancePanelView,
  CommunityNodePanelView,
  ConnectivityPanelView,
  DiscoveryPanelView,
} from './types';

export const appearancePanelFixture: AppearancePanelView = {
  selectedTheme: 'dark',
  options: [
    {
      value: 'dark',
      label: 'Dark',
      description: 'High-contrast solid surfaces for low-light work.',
    },
    {
      value: 'light',
      label: 'Light',
      description: 'Brighter solid surfaces for daytime readability.',
    },
  ],
};

export const connectivityPanelFixture: ConnectivityPanelView = {
  status: 'ready',
  summaryLabel: 'connected',
  panelError: null,
  metrics: [
    { label: 'Connected', value: 'yes', tone: 'accent' },
    { label: 'Peers', value: '2' },
    { label: 'Pending', value: '0' },
  ],
  diagnostics: [
    { label: 'Configured Peers', value: 'peer-a, peer-b', monospace: true },
    { label: 'Connection Detail', value: 'Connected to all configured peers' },
    { label: 'Effective Peers', value: 'peer-a, relay-peer', monospace: true },
    { label: 'Last Error', value: 'none' },
  ],
  localPeerTicket: 'peer1@127.0.0.1:7777',
  peerTicketInput: '',
  topics: [
    {
      topic: 'kukuri:topic:demo',
      summary: 'joined / peers: 2',
      lastReceivedLabel: '12:45:11',
      expectedPeerCount: 2,
      missingPeerCount: 0,
      statusDetail: 'Connected to all configured peers for this topic',
      connectedPeersLabel: 'peer-a, peer-b',
      relayAssistedPeersLabel: 'relay-peer',
      configuredPeersLabel: 'peer-a, peer-b',
      missingPeersLabel: 'none',
      lastError: null,
    },
    {
      topic: 'kukuri:topic:relay',
      summary: 'relay-assisted / peers: 1',
      lastReceivedLabel: 'no events',
      expectedPeerCount: 0,
      missingPeerCount: 0,
      statusDetail: 'relay-assisted sync available via 1 peer(s)',
      connectedPeersLabel: 'none',
      relayAssistedPeersLabel: 'relay-peer',
      configuredPeersLabel: 'none',
      missingPeersLabel: 'none',
      lastError: 'timed out waiting for gossip topic join',
    },
  ],
};

export const discoveryPanelFixture: DiscoveryPanelView = {
  status: 'ready',
  summaryLabel: 'seeded_dht',
  panelError: null,
  metrics: [
    { label: 'Mode', value: 'seeded_dht' },
    { label: 'Connect', value: 'direct_or_relay', tone: 'accent' },
    { label: 'Env Lock', value: 'no' },
  ],
  diagnostics: [
    { label: 'Local Endpoint ID', value: 'local-endpoint-a', monospace: true },
    { label: 'Connected Peers', value: 'peer-a', monospace: true },
    { label: 'Relay-assisted Peers', value: 'relay-peer', monospace: true },
    { label: 'Manual Ticket Peers', value: 'peer-ticket-1', monospace: true },
    { label: 'Community Bootstrap Peers', value: 'bootstrap-peer-1', monospace: true },
    { label: 'Configured Seed IDs', value: 'seed-peer-1', monospace: true },
    { label: 'Discovery Error', value: 'none' },
  ],
  seedPeersInput: 'seed-peer-1\nseed-peer-2@127.0.0.1:7777',
  seedPeersMessage: 'Editing stays enabled because discovery is not env-locked.',
  seedPeersMessageTone: 'default',
  envLocked: false,
};

export const communityNodePanelFixture: CommunityNodePanelView = {
  status: 'ready',
  summaryLabel: '2 configured',
  panelError: null,
  baseUrlsInput: 'https://api.kukuri.app\nhttps://community.example.com',
  editorMessage: 'Save nodes before authenticating.',
  editorMessageTone: 'default',
  nodes: [
    {
      baseUrl: 'https://api.kukuri.app',
      diagnostics: [
        { label: 'Auth', value: 'yes (1711324800000)' },
        { label: 'Consent', value: 'accepted' },
        { label: 'Connectivity URLs', value: 'https://api.kukuri.app', monospace: true },
        { label: 'Session Activation', value: 'active on current session' },
        { label: 'Next Step', value: 'connectivity urls active on current session' },
        { label: 'Last Error', value: 'none' },
      ],
      lastError: null,
    },
    {
      baseUrl: 'https://community.example.com',
      diagnostics: [
        { label: 'Auth', value: 'no' },
        { label: 'Consent', value: 'unknown' },
        { label: 'Connectivity URLs', value: 'not resolved', monospace: true },
        { label: 'Session Activation', value: 'not authenticated' },
        { label: 'Next Step', value: 'authenticate this node' },
        { label: 'Last Error', value: 'failed to refresh community node', tone: 'danger' },
      ],
      lastError: 'failed to refresh community node',
    },
  ],
};
