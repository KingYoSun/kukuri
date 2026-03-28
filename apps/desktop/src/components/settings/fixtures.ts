import type {
  AppearancePanelView,
  CommunityNodePanelView,
  ConnectivityPanelView,
  DiscoveryPanelView,
} from './types';
import i18n from '@/i18n';
import { formatLocalizedTime, getResolvedLocale } from '@/i18n/format';

export function createAppearancePanelFixture(): AppearancePanelView {
  return {
    selectedTheme: 'dark',
    selectedLocale: getResolvedLocale(i18n.resolvedLanguage),
    options: [
      {
        value: 'dark',
        label: i18n.t('settings:appearance.themeOptions.dark.label'),
        description: i18n.t('settings:appearance.themeOptions.dark.description'),
      },
      {
        value: 'light',
        label: i18n.t('settings:appearance.themeOptions.light.label'),
        description: i18n.t('settings:appearance.themeOptions.light.description'),
      },
    ],
    localeOptions: [
      { value: 'en', label: i18n.t('settings:appearance.languageOptions.en') },
      { value: 'ja', label: i18n.t('settings:appearance.languageOptions.ja') },
      { value: 'zh-CN', label: i18n.t('settings:appearance.languageOptions.zh-CN') },
    ],
  };
}

export const appearancePanelFixture = createAppearancePanelFixture();

export function createConnectivityPanelFixture(): ConnectivityPanelView {
  return {
    status: 'ready',
    summaryLabel: i18n.t('common:states.connected'),
    panelError: null,
    metrics: [
      { label: i18n.t('settings:connectivity.metrics.connected'), value: i18n.t('common:states.yes'), tone: 'accent' },
      { label: i18n.t('settings:connectivity.metrics.peers'), value: '2' },
      { label: i18n.t('settings:connectivity.metrics.pending'), value: '0' },
    ],
    diagnostics: [
      {
        label: i18n.t('settings:connectivity.diagnostics.configuredPeers'),
        value: 'peer-a, peer-b',
        monospace: true,
      },
      {
        label: i18n.t('settings:connectivity.diagnostics.connectionDetail'),
        value: 'Connected to all configured peers',
      },
      {
        label: i18n.t('settings:connectivity.diagnostics.effectivePeers'),
        value: 'peer-a, relay-peer',
        monospace: true,
      },
      { label: i18n.t('settings:connectivity.diagnostics.lastError'), value: i18n.t('common:fallbacks.none') },
    ],
    localPeerTicket: 'peer1@127.0.0.1:7777',
    peerTicketInput: '',
    topics: [
      {
        topic: 'kukuri:topic:demo',
        summary: i18n.t('settings:connectivity.summary', {
          status: i18n.t('common:states.joined'),
          count: 2,
        }),
        lastReceivedLabel: formatLocalizedTime('2026-03-28T12:45:11Z'),
        expectedPeerCount: 2,
        missingPeerCount: 0,
        statusDetail: 'Connected to all configured peers for this topic',
        connectedPeersLabel: 'peer-a, peer-b',
        relayAssistedPeersLabel: 'relay-peer',
        configuredPeersLabel: 'peer-a, peer-b',
        missingPeersLabel: i18n.t('common:fallbacks.none'),
        lastError: null,
      },
      {
        topic: 'kukuri:topic:relay',
        summary: i18n.t('settings:connectivity.summary', {
          status: i18n.t('common:states.relayAssisted'),
          count: 1,
        }),
        lastReceivedLabel: i18n.t('common:fallbacks.noEvents'),
        expectedPeerCount: 0,
        missingPeerCount: 0,
        statusDetail: 'relay-assisted sync available via 1 peer(s)',
        connectedPeersLabel: i18n.t('common:fallbacks.none'),
        relayAssistedPeersLabel: 'relay-peer',
        configuredPeersLabel: i18n.t('common:fallbacks.none'),
        missingPeersLabel: i18n.t('common:fallbacks.none'),
        lastError: 'timed out waiting for gossip topic join',
      },
    ],
  };
}

export const connectivityPanelFixture = createConnectivityPanelFixture();

export function createDiscoveryPanelFixture(): DiscoveryPanelView {
  return {
    status: 'ready',
    summaryLabel: 'seeded_dht',
    panelError: null,
    metrics: [
      { label: i18n.t('settings:discovery.metrics.mode'), value: 'seeded_dht' },
      { label: i18n.t('settings:discovery.metrics.connect'), value: 'direct_or_relay', tone: 'accent' },
      { label: i18n.t('settings:discovery.metrics.envLock'), value: i18n.t('common:states.no') },
    ],
    diagnostics: [
      { label: i18n.t('settings:discovery.diagnostics.localEndpointId'), value: 'local-endpoint-a', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.connectedPeers'), value: 'peer-a', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.relayAssistedPeers'), value: 'relay-peer', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.manualTicketPeers'), value: 'peer-ticket-1', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.communityBootstrapPeers'), value: 'bootstrap-peer-1', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.configuredSeedIds'), value: 'seed-peer-1', monospace: true },
      { label: i18n.t('settings:discovery.diagnostics.discoveryError'), value: i18n.t('common:fallbacks.none') },
    ],
    seedPeersInput: 'seed-peer-1\nseed-peer-2@127.0.0.1:7777',
    seedPeersMessage: 'Editing stays enabled because discovery is not env-locked.',
    seedPeersMessageTone: 'default',
    envLocked: false,
  };
}

export const discoveryPanelFixture = createDiscoveryPanelFixture();

export function createCommunityNodePanelFixture(): CommunityNodePanelView {
  return {
    status: 'ready',
    summaryLabel: i18n.t('settings:communityNode.summary', { count: 2 }),
    panelError: null,
    baseUrlsInput: 'https://api.kukuri.app\nhttps://community.example.com',
    editorMessage: 'Save nodes before authenticating.',
    editorMessageTone: 'default',
    nodes: [
      {
        baseUrl: 'https://api.kukuri.app',
        diagnostics: [
          { label: i18n.t('settings:communityNode.diagnostics.auth'), value: `${i18n.t('common:states.yes')} (1711324800000)` },
          { label: i18n.t('settings:communityNode.diagnostics.consent'), value: i18n.t('common:states.accepted') },
          {
            label: i18n.t('settings:communityNode.diagnostics.connectivityUrls'),
            value: 'https://api.kukuri.app',
            monospace: true,
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.sessionActivation'),
            value: i18n.t('settings:communityNode.values.activeOnCurrentSession'),
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.nextStep'),
            value: i18n.t('settings:communityNode.values.connectivityUrlsActiveOnCurrentSession'),
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.lastError'),
            value: i18n.t('common:fallbacks.none'),
          },
        ],
        lastError: null,
      },
      {
        baseUrl: 'https://community.example.com',
        diagnostics: [
          { label: i18n.t('settings:communityNode.diagnostics.auth'), value: i18n.t('common:states.no') },
          { label: i18n.t('settings:communityNode.diagnostics.consent'), value: i18n.t('common:states.unknown') },
          {
            label: i18n.t('settings:communityNode.diagnostics.connectivityUrls'),
            value: i18n.t('settings:communityNode.values.notResolved'),
            monospace: true,
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.sessionActivation'),
            value: i18n.t('settings:communityNode.values.notAuthenticated'),
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.nextStep'),
            value: i18n.t('settings:communityNode.values.authenticateThisNode'),
          },
          {
            label: i18n.t('settings:communityNode.diagnostics.lastError'),
            value: i18n.t('common:errors.failedToRefreshCommunityNode'),
            tone: 'danger',
          },
        ],
        lastError: i18n.t('common:errors.failedToRefreshCommunityNode'),
      },
    ],
  };
}

export const communityNodePanelFixture = createCommunityNodePanelFixture();
