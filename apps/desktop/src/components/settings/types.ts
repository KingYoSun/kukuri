import { type DesktopTheme } from '@/lib/theme';
import { type SupportedLocale } from '@/i18n';
import { type BookmarkedCustomReactionView, type CustomReactionAssetView } from '@/lib/api';

export type SettingsPanelStatus = 'loading' | 'ready' | 'error';

export type SettingsMetricView = {
  label: string;
  value: string;
  tone?: 'default' | 'accent' | 'warning' | 'danger';
};

export type SettingsDiagnosticItemView = {
  label: string;
  value: string;
  tone?: 'default' | 'danger';
  monospace?: boolean;
};

export type ConnectivityTopicDetailView = {
  topic: string;
  summary: string;
  lastReceivedLabel: string;
  expectedPeerCount: number;
  missingPeerCount: number;
  statusDetail: string;
  connectedPeersLabel: string;
  relayAssistedPeersLabel: string;
  configuredPeersLabel: string;
  missingPeersLabel: string;
  lastError?: string | null;
};

export type ConnectivityPanelView = {
  status: SettingsPanelStatus;
  summaryLabel: string;
  panelError?: string | null;
  metrics: SettingsMetricView[];
  diagnostics: SettingsDiagnosticItemView[];
  localPeerTicket: string;
  peerTicketInput: string;
  topics: ConnectivityTopicDetailView[];
};

export type DiscoveryPanelView = {
  status: SettingsPanelStatus;
  summaryLabel: string;
  panelError?: string | null;
  metrics: SettingsMetricView[];
  diagnostics: SettingsDiagnosticItemView[];
  seedPeersInput: string;
  seedPeersMessage?: string;
  seedPeersMessageTone?: 'default' | 'danger';
  envLocked: boolean;
};

// public manifest (#356) 由来の依存度 / capability scope / authority scope 表示。
export type CommunityNodeDependencyView = {
  // role / origin / manifest status / capability scope / authority scope を行として表示する。
  diagnostics: SettingsDiagnosticItemView[];
  // identity / profile / social graph が node-owned ではない等、常に表示する責任境界の説明。
  boundaryNotes: string[];
  // manifest fetch が失敗した場合のエラー（client は default node へ fallback しない）。
  manifestError?: string | null;
};

// per-node consent ダイアログ（#384）で表示する 1 ポリシー分の行。
export type CommunityNodeConsentPolicyView = {
  policySlug: string;
  title: string;
  body: string;
  policyVersion: number;
  required: boolean;
  acceptedAtLabel: string | null;
  // 版が上がって再同意が必要な「更新」状態か。
  updated: boolean;
  previouslyAcceptedVersion: number | null;
};

// per-node consent ダイアログ全体の表示状態。
export type CommunityNodeConsentView = {
  // 認証済みでないと consent を取得・受諾できない。
  authenticated: boolean;
  // consent_state を一度でも取得できているか（未取得なら本文表示前に取得を促す）。
  loaded: boolean;
  allRequiredAccepted: boolean;
  // 更新による未同意（再同意要求）が 1 つでもあるか。
  hasPendingUpdate: boolean;
  policies: CommunityNodeConsentPolicyView[];
};

export type CommunityNodeEntryView = {
  id: string;
  baseUrl: string;
  autoApprove: boolean;
  saved: boolean;
  diagnostics: SettingsDiagnosticItemView[];
  dependency: CommunityNodeDependencyView;
  consent: CommunityNodeConsentView;
  lastError?: string | null;
};

export type CommunityNodePanelView = {
  status: SettingsPanelStatus;
  summaryLabel: string;
  panelError?: string | null;
  editorMessage?: string;
  editorMessageTone?: 'default' | 'danger';
  nodes: CommunityNodeEntryView[];
};

export type AppearanceOptionView = {
  value: DesktopTheme;
  label: string;
  description: string;
};

export type LocaleOptionView = {
  value: SupportedLocale;
  label: string;
};

export type AppearancePanelView = {
  selectedTheme: DesktopTheme;
  selectedLocale: SupportedLocale;
  options: AppearanceOptionView[];
  localeOptions: LocaleOptionView[];
};

export type ReactionsPanelView = {
  status: SettingsPanelStatus;
  summaryLabel: string;
  panelError?: string | null;
  ownedAssets: CustomReactionAssetView[];
  bookmarkedAssets: BookmarkedCustomReactionView[];
};
