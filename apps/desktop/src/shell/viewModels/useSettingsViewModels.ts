import { useMemo } from 'react';

import { buildCommunityNodeDependencyView } from '@/components/settings/communityNodeDependency';
import type {
  AppearancePanelView,
  CommunityNodePanelView,
  ConnectivityPanelView,
  DiscoveryPanelView,
  ReactionsPanelView,
} from '@/components/settings/types';
import type { TopicSyncStatus } from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import type { DesktopTheme } from '@/lib/theme';
import {
  communityNodeAuthLabel,
  communityNodeConnectivityUrlsLabel,
  communityNodeConsentLabel,
  communityNodeConsentView,
  communityNodeNextStepLabel,
  communityNodeRetryAfterLabel,
  communityNodeSessionPhaseLabel,
  communityNodeSessionActivationLabel,
  formatCount,
  formatLastReceivedLabel,
  formatListLabel,
  syncStatusBadgeLabel,
  topicConnectionLabel,
  translateTopicConnectionText,
} from '@/shell/presentation';
import type { DesktopShellState } from '@/shell/store';

type UseSettingsViewModelsArgs = {
  bookmarkedReactionAssets: DesktopShellState['bookmarkedReactionAssets'];
  communityNodeConfig: DesktopShellState['communityNodeConfig'];
  communityNodeEditorDirty: DesktopShellState['communityNodeEditorDirty'];
  communityNodeError: DesktopShellState['communityNodeError'];
  communityNodeInput: DesktopShellState['communityNodeInput'];
  communityNodeManifests: DesktopShellState['communityNodeManifests'];
  communityNodeStatuses: DesktopShellState['communityNodeStatuses'];
  discoveryConfig: DesktopShellState['discoveryConfig'];
  discoveryEditorDirty: DesktopShellState['discoveryEditorDirty'];
  discoveryError: DesktopShellState['discoveryError'];
  discoverySeedInput: DesktopShellState['discoverySeedInput'];
  error: DesktopShellState['error'];
  locale: SupportedLocale;
  localPeerTicket: DesktopShellState['localPeerTicket'];
  ownedReactionAssets: DesktopShellState['ownedReactionAssets'];
  peerTicket: DesktopShellState['peerTicket'];
  reactionPanelState: DesktopShellState['reactionPanelState'];
  syncStatus: DesktopShellState['syncStatus'];
  t: (key: string, options?: Record<string, unknown>) => string;
  theme: DesktopTheme;
  topicDiagnostics: Record<string, TopicSyncStatus>;
  trackedTopics: DesktopShellState['trackedTopics'];
};

// settings section(connectivity / appearance / discovery / community-node /
// reactions の 5 panel)の projection(Q3 T5 の分割先)。
export function useSettingsViewModels({
  bookmarkedReactionAssets,
  communityNodeConfig,
  communityNodeEditorDirty,
  communityNodeError,
  communityNodeInput,
  communityNodeManifests,
  communityNodeStatuses,
  discoveryConfig,
  discoveryEditorDirty,
  discoveryError,
  discoverySeedInput,
  error,
  locale,
  localPeerTicket,
  ownedReactionAssets,
  peerTicket,
  reactionPanelState,
  syncStatus,
  t,
  theme,
  topicDiagnostics,
  trackedTopics,
}: UseSettingsViewModelsArgs) {
  const communityNodeStatusByBaseUrl = useMemo(
    () =>
      Object.fromEntries(communityNodeStatuses.map((status) => [status.base_url, status])) as Record<
        string,
        (typeof communityNodeStatuses)[number]
      >,
    [communityNodeStatuses]
  );

  const effectivePeerIds = useMemo(
    () =>
      [
        ...new Set([
          ...syncStatus.topic_diagnostics.flatMap((diagnostic) => diagnostic.connected_peers),
          ...syncStatus.discovery.docs_assist_peer_ids,
          ...syncStatus.discovery.blob_assist_peer_ids,
        ]),
      ],
    [
      syncStatus.discovery.blob_assist_peer_ids,
      syncStatus.discovery.docs_assist_peer_ids,
      syncStatus.topic_diagnostics,
    ]
  );

  const connectivityPanelView = useMemo<ConnectivityPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatusBadgeLabel(syncStatus),
      panelError: error,
      metrics: [
        {
          label: t('settings:connectivity.metrics.connected'),
          value: syncStatus.connected ? t('common:states.yes') : t('common:states.no'),
          tone: syncStatus.connected ? 'accent' : 'warning',
        },
        {
          label: t('settings:connectivity.metrics.peers'),
          value: formatCount(syncStatus.peer_count),
        },
        {
          label: t('settings:connectivity.metrics.pending'),
          value: formatCount(syncStatus.pending_events),
          tone: syncStatus.pending_events > 0 ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:connectivity.diagnostics.configuredPeers'),
          value: formatListLabel(syncStatus.configured_peers),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.connectionDetail'),
          value: syncStatus.status_detail || t('settings:connectivity.summaryDetailFallback'),
        },
        {
          label: t('settings:connectivity.diagnostics.effectivePeers'),
          value: formatListLabel(effectivePeerIds),
          monospace: true,
        },
        {
          label: t('settings:connectivity.diagnostics.lastError'),
          value: syncStatus.last_error ?? t('common:fallbacks.none'),
          tone: syncStatus.last_error ? 'danger' : 'default',
        },
      ],
      localPeerTicket: localPeerTicket ?? '',
      peerTicketInput: peerTicket,
      topics: trackedTopics.map((topic) => {
        const diagnostic = topicDiagnostics[topic];
        return {
          topic,
          summary: t('settings:connectivity.summary', {
            status: translateTopicConnectionText(topicConnectionLabel(diagnostic)),
            count: diagnostic?.peer_count ?? 0,
          }),
          lastReceivedLabel: formatLastReceivedLabel(diagnostic?.last_received_at, locale),
          expectedPeerCount: diagnostic?.configured_peer_ids.length ?? 0,
          missingPeerCount: diagnostic?.missing_peer_ids.length ?? 0,
          statusDetail:
            diagnostic?.status_detail ?? t('settings:connectivity.summaryDetailFallback'),
          connectedPeersLabel: formatListLabel(diagnostic?.connected_peers ?? []),
          relayAssistedPeersLabel: formatListLabel(diagnostic?.docs_assist_peer_ids ?? []),
          configuredPeersLabel: formatListLabel(diagnostic?.configured_peer_ids ?? []),
          missingPeersLabel: formatListLabel(diagnostic?.missing_peer_ids ?? []),
          lastError: diagnostic?.last_error ?? null,
        };
      }),
    }),
    [
      effectivePeerIds,
      error,
      localPeerTicket,
      locale,
      peerTicket,
      syncStatus,
      t,
      topicDiagnostics,
      trackedTopics,
    ]
  );

  const appearancePanelView = useMemo<AppearancePanelView>(
    () => ({
      selectedTheme: theme,
      selectedLocale: locale,
      options: [
        {
          value: 'dark',
          label: t('settings:appearance.themeOptions.dark.label'),
          description: t('settings:appearance.themeOptions.dark.description'),
        },
        {
          value: 'light',
          label: t('settings:appearance.themeOptions.light.label'),
          description: t('settings:appearance.themeOptions.light.description'),
        },
      ],
      localeOptions: [
        {
          value: 'en',
          label: t('settings:appearance.languageOptions.en'),
        },
        {
          value: 'ja',
          label: t('settings:appearance.languageOptions.ja'),
        },
        {
          value: 'zh-CN',
          label: t('settings:appearance.languageOptions.zh-CN'),
        },
      ],
    }),
    [locale, t, theme]
  );

  const discoveryPanelView = useMemo<DiscoveryPanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: syncStatus.discovery.mode,
      panelError: null,
      metrics: [
        { label: t('settings:discovery.metrics.mode'), value: syncStatus.discovery.mode },
        {
          label: t('settings:discovery.metrics.connect'),
          value: syncStatus.discovery.connect_mode,
          tone: syncStatus.discovery.connect_mode === 'direct_or_relay' ? 'accent' : 'default',
        },
        {
          label: t('settings:discovery.metrics.envLock'),
          value: discoveryConfig.env_locked ? t('common:states.yes') : t('common:states.no'),
          tone: discoveryConfig.env_locked ? 'warning' : 'default',
        },
      ],
      diagnostics: [
        {
          label: t('settings:discovery.diagnostics.localEndpointId'),
          value: syncStatus.discovery.local_endpoint_id || t('common:fallbacks.unknown'),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.connectedPeers'),
          value: formatListLabel(syncStatus.discovery.connected_peer_ids),
          monospace: true,
        },
        {
          label: 'Docs Assist Peers',
          value: formatListLabel(syncStatus.discovery.docs_assist_peer_ids),
          monospace: true,
        },
        {
          label: 'Blob Assist Peers',
          value: formatListLabel(syncStatus.discovery.blob_assist_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.manualTicketPeers'),
          value: formatListLabel(syncStatus.discovery.manual_ticket_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.communityBootstrapPeers'),
          value: formatListLabel(syncStatus.discovery.bootstrap_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.configuredSeedIds'),
          value: formatListLabel(syncStatus.discovery.configured_seed_peer_ids),
          monospace: true,
        },
        {
          label: t('settings:discovery.diagnostics.discoveryError'),
          value: discoveryError ?? syncStatus.discovery.last_discovery_error ?? t('common:fallbacks.none'),
          tone:
            discoveryError || syncStatus.discovery.last_discovery_error ? 'danger' : 'default',
        },
      ],
      seedPeersInput: discoverySeedInput,
      seedPeersMessage: discoveryConfig.env_locked
        ? t('settings:discovery.messages.viewLocked')
        : discoveryEditorDirty
          ? t('settings:discovery.messages.unsaved')
          : t('settings:discovery.messages.saved'),
      seedPeersMessageTone: discoveryConfig.env_locked ? ('default' as const) : ('default' as const),
      envLocked: discoveryConfig.env_locked,
    }),
    [
      discoveryConfig.env_locked,
      discoveryEditorDirty,
      discoveryError,
      discoverySeedInput,
      syncStatus.discovery.blob_assist_peer_ids,
      syncStatus.discovery.bootstrap_seed_peer_ids,
      syncStatus.discovery.configured_seed_peer_ids,
      syncStatus.discovery.connect_mode,
      syncStatus.discovery.connected_peer_ids,
      syncStatus.discovery.docs_assist_peer_ids,
      syncStatus.discovery.last_discovery_error,
      syncStatus.discovery.local_endpoint_id,
      syncStatus.discovery.manual_ticket_peer_ids,
      syncStatus.discovery.mode,
      t,
    ]
  );

  const communityNodePanelView = useMemo<CommunityNodePanelView>(
    () => ({
      status: 'ready' as const,
      summaryLabel: t('settings:communityNode.summary', { count: communityNodeInput.length }),
      panelError: communityNodeError,
      editorMessage: communityNodeEditorDirty
        ? t('settings:communityNode.editorMessage.unsaved')
        : t('settings:communityNode.editorMessage.saved'),
      editorMessageTone: 'default' as const,
      nodes: communityNodeInput.map((node) => {
        const saved =
          communityNodeConfig.nodes.find(
            (candidate) =>
              candidate.base_url === node.base_url &&
              (candidate.auto_approve ?? false) === node.auto_approve
          ) != null;
        const status = communityNodeStatusByBaseUrl[node.base_url];
        return {
          id: node.id,
          baseUrl: node.base_url,
          autoApprove: node.auto_approve,
          saved,
          diagnostics: [
            {
              label: t('settings:communityNode.diagnostics.autoApprove'),
              value: node.auto_approve ? t('common:states.yes') : t('common:states.no'),
            },
            {
              label: t('settings:communityNode.diagnostics.auth'),
              value: communityNodeAuthLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.consent'),
              value: communityNodeConsentLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.connectivityUrls'),
              value: communityNodeConnectivityUrlsLabel(status),
              monospace: true,
            },
            {
              label: t('settings:communityNode.diagnostics.sessionPhase'),
              value: communityNodeSessionPhaseLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.retryAfter'),
              value: communityNodeRetryAfterLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.sessionActivation'),
              value: communityNodeSessionActivationLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.nextStep'),
              value: communityNodeNextStepLabel(status),
            },
            {
              label: t('settings:communityNode.diagnostics.lastError'),
              value: status?.last_error ?? t('common:fallbacks.none'),
              tone: status?.last_error ? 'danger' : 'default',
            },
          ],
          dependency: buildCommunityNodeDependencyView(
            communityNodeManifests[node.base_url],
            t
          ),
          consent: communityNodeConsentView(status),
          inviteCodeSaved: status?.invite_code_saved ?? false,
          admissionRejectionCode: status?.admission_rejection?.code ?? null,
          lastError: status?.last_error ?? null,
        };
      }),
    }),
    [
      communityNodeConfig.nodes,
      communityNodeEditorDirty,
      communityNodeError,
      communityNodeInput,
      communityNodeManifests,
      communityNodeStatusByBaseUrl,
      t,
    ]
  );

  const reactionsPanelView = useMemo<ReactionsPanelView>(
    () => ({
      status: reactionPanelState.status,
      summaryLabel: t('settings:reactions.summary', {
        owned: ownedReactionAssets.length,
        saved: bookmarkedReactionAssets.length,
      }),
      panelError: reactionPanelState.error,
      ownedAssets: ownedReactionAssets,
      bookmarkedAssets: bookmarkedReactionAssets,
    }),
    [
      bookmarkedReactionAssets,
      ownedReactionAssets,
      reactionPanelState.error,
      reactionPanelState.status,
      t,
    ]
  );

  return {
    communityNodeStatusByBaseUrl,
    effectivePeerIds,
    connectivityPanelView,
    appearancePanelView,
    discoveryPanelView,
    communityNodePanelView,
    reactionsPanelView,
  };
}
