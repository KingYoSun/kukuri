import { type SettingsSection, type ShellChromeState } from '@/components/shell/types';
import { readDeveloperMode } from '@/lib/developerMode';
import { parseHashRouteLocation } from '@/shell/routes';

/// shell chrome(ナビ・設定ドロワー)とルート記憶・全体エラー(WP-H6 PR3 のドメインスライス)。
export type ChromeSliceState = {
  error: string | null;
  lastNonNotificationsRoute: string | null;
  developerModeEnabled: boolean;
  shellChromeState: ShellChromeState;
};

const DEFAULT_SETTINGS_SECTION: SettingsSection = 'connectivity';

function parseInitialSettingsSection(): {
  activeSettingsSection: SettingsSection;
  settingsOpen: boolean;
} {
  if (typeof window === 'undefined') {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  const { search } = parseHashRouteLocation(window.location.hash);
  if (!search) {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  const requestedSection = new URLSearchParams(search).get('settings');
  if (
    requestedSection !== 'about' &&
    requestedSection !== 'appearance' &&
    requestedSection !== 'connectivity' &&
    requestedSection !== 'discovery' &&
    requestedSection !== 'community-node' &&
    requestedSection !== 'reactions' &&
    requestedSection !== 'release' &&
    requestedSection !== 'developer'
  ) {
    return {
      activeSettingsSection: DEFAULT_SETTINGS_SECTION,
      settingsOpen: false,
    };
  }

  return {
    activeSettingsSection: requestedSection,
    settingsOpen: true,
  };
}

export function createInitialChromeSlice(): ChromeSliceState {
  const initialSettingsState = parseInitialSettingsSection();
  return {
    error: null,
    lastNonNotificationsRoute: null,
    developerModeEnabled: readDeveloperMode(),
    shellChromeState: {
      activePrimarySection: 'timeline',
      // timelineView の正本は ColumnState.timelineView(workspace slice)。ここは
      // 「active(route focus 中)Timeline Column の view の投影」で、route 同期や
      // useShellDialogs 等の既存参照との互換のために残している(Issue #765)。
      timelineView: 'feed',
      activeSettingsSection: initialSettingsState.activeSettingsSection,
      profileMode: 'overview',
      profileConnectionsView: 'following',
      navOpen: false,
      settingsOpen: initialSettingsState.settingsOpen,
    },
  };
}
