import { AppearancePanel } from '@/components/settings/AppearancePanel';
import { CommunityNodePanel } from '@/components/settings/CommunityNodePanel';
import { ConnectivityPanel } from '@/components/settings/ConnectivityPanel';
import { DiscoveryPanel } from '@/components/settings/DiscoveryPanel';
import { ReleasePanel } from '@/components/settings/ReleasePanel';
import { ReactionsPanel } from '@/components/settings/ReactionsPanel';
import { SettingsDrawer } from '@/components/shell/SettingsDrawer';

import type { SupportedLocale } from '@/i18n';
import type { CustomReactionCropRect } from '@/lib/api';
import type { DesktopTheme } from '@/lib/theme';
import { communityNodesToDraftNodes, seedPeersToEditorValue } from '@/shell/selectors';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import type { SyncRoute } from '@/shell/actions/shared';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;

type DesktopShellSettingsDrawerProps = {
  drawerId: string;
  onThemeChange: (theme: DesktopTheme) => void;
  onLocaleChange: (locale: SupportedLocale) => void;
  syncRoute: SyncRoute;
  setSettingsOpen: (open: boolean, focusTrigger?: boolean) => void;
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
  handleFetchCommunityNodeConsents: (baseUrl: string) => Promise<void>;
  handleAcceptCommunityNodeConsents: (baseUrl: string) => Promise<void>;
  handleRefreshCommunityNode: (baseUrl: string) => Promise<void>;
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
  drawerId,
  onThemeChange,
  onLocaleChange,
  syncRoute,
  setSettingsOpen,
  viewModels,
  handleImportPeer,
  handleSaveDiscoverySeeds,
  handleSaveCommunityNodes,
  handleClearCommunityNodes,
  handleAuthenticateCommunityNode,
  handleFetchCommunityNodeConsents,
  handleAcceptCommunityNodeConsents,
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
    communityNodeConfig,
    communityNodeEditorDirty,
    discoveryConfig,
    discoveryEditorDirty,
    mediaObjectUrls,
    reactionCreatePending,
    shellChromeState,
  } = useDesktopShellStore();
  const setPeerTicket = useDesktopShellFieldSetter('peerTicket');
  const setDiscoverySeedInput = useDesktopShellFieldSetter('discoverySeedInput');
  const setDiscoveryEditorDirty = useDesktopShellFieldSetter('discoveryEditorDirty');
  const setDiscoveryError = useDesktopShellFieldSetter('discoveryError');
  const setCommunityNodeInput = useDesktopShellFieldSetter('communityNodeInput');
  const setCommunityNodeEditorDirty = useDesktopShellFieldSetter('communityNodeEditorDirty');
  const setCommunityNodeError = useDesktopShellFieldSetter('communityNodeError');
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');

  const settingsSections = [
    {
      ...settingsSectionCopy[0],
      content: (
        <AppearancePanel
          view={appearancePanelView}
          onThemeChange={onThemeChange}
          onLocaleChange={onLocaleChange}
        />
      ),
    },
    {
      ...settingsSectionCopy[1],
      content: (
        <ConnectivityPanel
          view={connectivityPanelView}
          onPeerTicketInputChange={setPeerTicket}
          onImportPeer={() => void handleImportPeer()}
        />
      ),
    },
    {
      ...settingsSectionCopy[2],
      content: (
        <DiscoveryPanel
          view={discoveryPanelView}
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
      ...settingsSectionCopy[3],
      content: (
        <CommunityNodePanel
          view={communityNodePanelView}
          saveDisabled={!communityNodeEditorDirty}
          resetDisabled={!communityNodeEditorDirty}
          clearDisabled={communityNodeConfig.nodes.length === 0}
          nodeActionsDisabled={communityNodeEditorDirty}
          onAddNode={() => {
            setCommunityNodeInput((current) => [
              ...current,
              {
                id: createCommunityNodeDraftId(),
                base_url: '',
                auto_approve: false,
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
          onNodeAutoApproveChange={(id, value) => {
            setCommunityNodeInput((current) =>
              current.map((node) => (node.id === id ? { ...node, auto_approve: value } : node))
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
          onFetchConsents={(baseUrl) => void handleFetchCommunityNodeConsents(baseUrl)}
          onAcceptConsents={(baseUrl) => void handleAcceptCommunityNodeConsents(baseUrl)}
          onRefresh={(baseUrl) => void handleRefreshCommunityNode(baseUrl)}
          onClearToken={(baseUrl) => void handleClearCommunityNodeToken(baseUrl)}
        />
      ),
    },
    {
      ...settingsSectionCopy[4],
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
      ...settingsSectionCopy[5],
      content: <ReleasePanel />,
    },
  ];

  return (
    <SettingsDrawer
      drawerId={drawerId}
      open={shellChromeState.settingsOpen}
      onOpenChange={(open) => setSettingsOpen(open, !open)}
      activeSection={shellChromeState.activeSettingsSection}
      onSectionChange={(section) => {
        setShellChromeState((current) => ({
          ...current,
          activeSettingsSection: section,
        }));
        syncRoute('replace', {
          settingsOpen: true,
          settingsSection: section,
        });
      }}
      sections={settingsSections}
    />
  );
}
