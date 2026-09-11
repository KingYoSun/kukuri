import { AboutPanel } from '@/components/settings/AboutPanel';
import { AccountKeyPanel } from '@/components/settings/AccountKeyPanel';
import { AppearancePanel } from '@/components/settings/AppearancePanel';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DeveloperPanel } from '@/components/settings/DeveloperPanel';
import { DeviceBackupPanel } from '@/components/settings/DeviceBackupPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import { ReleasePanel } from '@/components/settings/ReleasePanel';
import { ReactionsPanel } from '@/components/settings/ReactionsPanel';
import { SafetyPanel } from '@/components/settings/SafetyPanel';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';
import type { PrimarySection, ProfileConnectionsView, SettingsSection } from '@/components/shell/types';

import type { SupportedLocale } from '@/i18n';
import type { CustomReactionCropRect, DesktopApi } from '@/lib/api';
import type { FetchCommunityNodePolicyView, AcceptCommunityNodePolicyView } from '@/shell/actions/useCommunityNodePolicyDialog';
import {
  eligibleCommunityIndexNodes,
  resolveCommunityIndexNodePreference,
} from '@/lib/api/communityIndex';
import { writeDeveloperMode } from '@/lib/developerMode';
import type { DesktopTheme } from '@/lib/theme';
import { communityNodesToDraftNodes, seedPeersToEditorValue } from '@/shell/presentation';
import {
  SHELL_SETTINGS_ID,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import type { SyncRoute } from '@/shell/actions/shared';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import { useShallow } from 'zustand/react/shallow';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;

type DesktopShellSettingsDrawerProps = {
  api: DesktopApi;
  onRefreshDiagnostics: () => void;
  onThemeChange: (theme: DesktopTheme) => void;
  onLocaleChange: (locale: SupportedLocale) => void;
  localeSaveFailed?: boolean;
  syncRoute: SyncRoute;
  setSettingsOpen: (open: boolean, focusTrigger?: boolean) => void;
  focusPrimarySection: (section: PrimarySection) => void;
  openProfileConnections: (view: ProfileConnectionsView) => void;
  viewModels: Pick<
    ViewModels,
    | 'settingsSectionCopy'
    | 'appearancePanelView'
    | 'connectivityPanelView'
    | 'discoveryPanelView'
    | 'communityNodePanelView'
    | 'reactionsPanelView'
  >;
  handleImportPeer: () => Promise<void>;
  handleSaveDiscoverySeeds: () => Promise<void>;
  handleSaveCommunityNodes: () => Promise<void>;
  handleClearCommunityNodes: () => Promise<void>;
  handleAuthenticateCommunityNode: (baseUrl: string) => Promise<void>;
  handleSetCommunityNodeInviteCode: (baseUrl: string, inviteCode: string) => Promise<void>;
  handleFetchCommunityNodeConsents: FetchCommunityNodePolicyView;
  handleAcceptCommunityNodeConsents: AcceptCommunityNodePolicyView;
  handleWithdrawCommunityNodeConsents: (baseUrl: string) => Promise<void>;
  handleRefreshCommunityNode: (baseUrl: string) => Promise<boolean | void>;
  handleClearCommunityNodeToken: (baseUrl: string) => Promise<void>;
  handleCreateCustomReactionAsset: (
    file: File,
    cropRect: CustomReactionCropRect,
    searchKey: string
  ) => Promise<void>;
  handleRemoveBookmarkedCustomReaction: (assetId: string) => Promise<void>;
};

function createCommunityNodeDraftId(): string {
  return `community-node-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function DesktopShellSettingsDrawer({
  api,
  onRefreshDiagnostics,
  onThemeChange,
  onLocaleChange,
  localeSaveFailed,
  syncRoute,
  setSettingsOpen,
  focusPrimarySection,
  openProfileConnections,
  viewModels,
  handleImportPeer,
  handleSaveDiscoverySeeds,
  handleSaveCommunityNodes,
  handleClearCommunityNodes,
  handleAuthenticateCommunityNode,
  handleSetCommunityNodeInviteCode,
  handleFetchCommunityNodeConsents,
  handleAcceptCommunityNodeConsents,
  handleWithdrawCommunityNodeConsents,
  handleRefreshCommunityNode,
  handleClearCommunityNodeToken,
  handleCreateCustomReactionAsset,
  handleRemoveBookmarkedCustomReaction,
}: DesktopShellSettingsDrawerProps) {
  const {
    settingsSectionCopy,
    appearancePanelView,
    connectivityPanelView,
    discoveryPanelView,
    communityNodePanelView,
    reactionsPanelView,
  } = viewModels;
  const {
    adultContentEnabled,
    communityNodeConfig,
    communityNodeEditorDirty,
    communityIndexNodePreference,
    communityNodeManifests,
    communityNodeStatuses,
    developerModeEnabled,
    discoveryConfig,
    discoveryEditorDirty,
    mediaObjectUrls,
    reactionCreatePending,
    shellChromeState,
  } = useDesktopShellStore(
    useShallow((s) => ({
      adultContentEnabled: s.adultContentEnabled,
      communityNodeConfig: s.communityNodeConfig,
      communityNodeEditorDirty: s.communityNodeEditorDirty,
      communityIndexNodePreference: s.communityIndexNodePreference,
      communityNodeManifests: s.communityNodeManifests,
      communityNodeStatuses: s.communityNodeStatuses,
      developerModeEnabled: s.developerModeEnabled,
      discoveryConfig: s.discoveryConfig,
      discoveryEditorDirty: s.discoveryEditorDirty,
      mediaObjectUrls: s.mediaObjectUrls,
      reactionCreatePending: s.reactionCreatePending,
      shellChromeState: s.shellChromeState,
    }))
  );
  const setPeerTicket = useDesktopShellFieldSetter('peerTicket');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setDiscoveryEditorDirty = useDesktopShellFieldSetter('discoveryEditorDirty');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeEditorDirty = useDesktopShellFieldSetter('communityNodeEditorDirty');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const setDeveloperModeEnabled = useDesktopShellFieldSetter('developerModeEnabled');
  const setAdultContentEnabled = useDesktopShellFieldSetter('adultContentEnabled');
  const patchState = useDesktopShellStore((s) => s.patchState);
  // #858: canonical は Rust 側。コマンド成功後の値だけを mirror する(失敗時は既定 OFF 側に倒れる)。
  const handleAdultContentEnabledChange = async (enabled: boolean) => {
    try {
      const settings = await api.setAdultContentDisplayEnabled(enabled);
      setAdultContentEnabled(settings.adult_content_enabled);
    } catch {
      setAdultContentEnabled(false);
    }
  };
  const eligibleIndexNodeBaseUrls = eligibleCommunityIndexNodes(
    communityNodeConfig,
    communityNodeStatuses,
    communityNodeManifests
  );

  const changeSettingsSection = (section: SettingsSection) => {
    setShellChromeState((current) => ({ ...current, activeSettingsSection: section }));
    syncRoute('replace', { settingsOpen: true, settingsSection: section });
  };

  const openDiagnosticSettings = (section: SettingsSection) => {
    changeSettingsSection(section);
    document.getElementById(`${SHELL_SETTINGS_ID}-section-${section}`)?.focus();
  };

  // #961: 設定からミュート／ブロック一覧へ移動する。一覧の正本はプロフィールの connections 画面で、
  // Profile Column を前面にしてから開く。
  const openSocialConnections = (view: ProfileConnectionsView) => {
    setSettingsOpen(false);
    focusPrimarySection('profile');
    openProfileConnections(view);
  };

  const settingsSections = [
    {
      ...settingsSectionCopy[0],
      content: <AboutPanel />,
    },
    {
      ...settingsSectionCopy[1],
      content: (
        <AppearancePanel
          view={appearancePanelView}
          onThemeChange={onThemeChange}
          onLocaleChange={onLocaleChange}
          localeSaveFailed={localeSaveFailed}
        />
      ),
    },
    {
      ...settingsSectionCopy[2],
      content: (
        <SafetyPanel
          adultContentEnabled={adultContentEnabled}
          onAdultContentEnabledChange={(enabled) => void handleAdultContentEnabledChange(enabled)}
          onOpenMutedUsers={() => openSocialConnections('muted')}
          onOpenBlockedUsers={() => openSocialConnections('blocking')}
        />
      ),
    },
    {
      ...settingsSectionCopy[3],
      content: (
        <ConnectivityPanel
          onRefreshDiagnostics={onRefreshDiagnostics}
          onOpenCommunityNode={() => openDiagnosticSettings('community-node')}
          view={connectivityPanelView}
          onPeerTicketInputChange={setPeerTicket}
          onImportPeer={() => void handleImportPeer()}
          showDiagnostics={developerModeEnabled}
        />
      ),
    },
    {
      ...settingsSectionCopy[4],
      content: (
        <DiscoveryPanel
          onRefreshDiagnostics={onRefreshDiagnostics}
          onOpenCommunityNode={() => openDiagnosticSettings('community-node')}
          view={discoveryPanelView}
          showDiagnostics={developerModeEnabled}
          saveDisabled={discoveryConfig.env_locked || !discoveryEditorDirty}
          resetDisabled={!discoveryEditorDirty}
          onSeedPeersChange={(value) => {
            setDiscoverySeedInput(value);
            setDiscoveryEditorDirty(true);
          }}
          onSave={() => void handleSaveDiscoverySeeds()}
          onReset={() => {
            setDiscoverySeedInput(seedPeersToEditorValue(discoveryConfig));
            setDiscoveryEditorDirty(false);
            setDiscoveryError(null);
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[5],
      content: (
        <CommunityNodePanel
          view={communityNodePanelView}
          showDiagnostics={developerModeEnabled}
          saveDisabled={!communityNodeEditorDirty}
          resetDisabled={!communityNodeEditorDirty}
          clearDisabled={communityNodeConfig.nodes.length === 0}
          nodeActionsDisabled={communityNodeEditorDirty}
          indexNodePreference={communityIndexNodePreference}
          eligibleIndexNodeBaseUrls={eligibleIndexNodeBaseUrls}
          onIndexNodePreferenceChange={(preference) => {
            const resolution = resolveCommunityIndexNodePreference(
              preference,
              communityNodeConfig.nodes.map((node) => node.base_url),
              eligibleIndexNodeBaseUrls
            );
            patchState({
              communityIndexNodePreference: resolution.preference,
              communityIndexNodeBaseUrl: resolution.selectedBaseUrl,
            });
          }}
          onAddNode={() => {
            setCommunityNodeInput((current) => [
              ...current,
              {
                id: createCommunityNodeDraftId(),
                base_url: '',
              },
            ]);
            setCommunityNodeEditorDirty(true);
          }}
          onNodeBaseUrlChange={(id, value) => {
            setCommunityNodeInput((current) =>
              current.map((node) => (node.id === id ? { ...node, base_url: value } : node))
            );
            setCommunityNodeEditorDirty(true);
          }}
          onRemoveNode={(id) => {
            setCommunityNodeInput((current) => current.filter((node) => node.id !== id));
            setCommunityNodeEditorDirty(true);
          }}
          onSaveNodes={() => void handleSaveCommunityNodes()}
          onReset={() => {
            setCommunityNodeInput(communityNodesToDraftNodes(communityNodeConfig));
            setCommunityNodeEditorDirty(false);
            setCommunityNodeError(null);
          }}
          onClearNodes={() => void handleClearCommunityNodes()}
          onAuthenticate={(baseUrl) => void handleAuthenticateCommunityNode(baseUrl)}
          onSubmitInviteCode={handleSetCommunityNodeInviteCode}
          onFetchConsents={(baseUrl, language) => handleFetchCommunityNodeConsents(baseUrl, language)}
          onAcceptConsents={(baseUrl, documents, language) =>
            handleAcceptCommunityNodeConsents(baseUrl, documents, language)
          }
          onWithdrawConsents={(baseUrl) => handleWithdrawCommunityNodeConsents(baseUrl)}
          onRefresh={handleRefreshCommunityNode}
          onClearToken={(baseUrl) => void handleClearCommunityNodeToken(baseUrl)}
          onGetRelationOptout={(baseUrl) => api.getCommunityNodeRelationOptout(baseUrl)}
          onSetRelationOptout={(baseUrl) => api.setCommunityNodeRelationOptout(baseUrl)}
          onClearRelationOptout={(baseUrl) => api.clearCommunityNodeRelationOptout(baseUrl)}
        />
      ),
    },
    {
      ...settingsSectionCopy[6],
      // Keep the local file/crop draft when visiting Appearance to change language.
      keepMounted: true,
      content: (
        <ReactionsPanel
          view={reactionsPanelView}
          creating={reactionCreatePending}
          mediaObjectUrls={mediaObjectUrls}
          onCreateAsset={(file, cropRect, searchKey) =>
            void handleCreateCustomReactionAsset(file, cropRect, searchKey)
          }
          onRemoveBookmark={handleRemoveBookmarkedCustomReaction}
        />
      ),
    },
    {
      ...settingsSectionCopy[7],
      content: <ReleasePanel showDiagnostics={developerModeEnabled} />,
    },
    {
      ...settingsSectionCopy[8],
      content: (
        <DeveloperPanel
          developerModeEnabled={developerModeEnabled}
          onDeveloperModeChange={(enabled) => {
            setDeveloperModeEnabled(enabled);
            writeDeveloperMode(enabled);
          }}
          onOpenDiagnostics={(section) => {
            changeSettingsSection(section);
            // The selected panel unmounts; keep keyboard focus on the destination nav.
            document.getElementById(`${SHELL_SETTINGS_ID}-section-${section}`)?.focus();
          }}
        />
      ),
    },
    {
      ...settingsSectionCopy[9],
      content: (
        <div className='space-y-5'>
          <DeviceBackupPanel />
          <AccountKeyPanel />
        </div>
      ),
    },
  ];

  return (
    <SettingsDrawer
      drawerId={SHELL_SETTINGS_ID}
      open={shellChromeState.settingsOpen}
      onOpenChange={(open) => setSettingsOpen(open, !open)}
      activeSection={shellChromeState.activeSettingsSection}
      onSectionChange={changeSettingsSection}
      sections={settingsSections}
    />
  );
}
