import { type DesktopTheme } from '@/lib/theme';
import { type SupportedLocale } from '@/i18n';

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

export type CommunityNodeEntryView = {
  baseUrl: string;
  diagnostics: SettingsDiagnosticItemView[];
  lastError?: string | null;
};

export type CommunityNodePanelView = {
  status: SettingsPanelStatus;
  summaryLabel: string;
  panelError?: string | null;
  baseUrlsInput: string;
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
