import type { ChannelRef, TimelineScope } from '@/lib/api';
import type {
  PrimarySection,
  ProfileConnectionsView,
  ProfileWorkspaceMode,
  SettingsSection,
  TimelineWorkspaceView,
} from '@/components/shell/types';

export type DesktopShellRouteOverrides = {
  activeTopic?: string;
  composeTarget?: ChannelRef;
  primarySection?: PrimarySection;
  profileMode?: ProfileWorkspaceMode;
  profileConnectionsView?: ProfileConnectionsView;
  selectedAuthorPubkey?: string | null;
  directMessagePaneOpen?: boolean;
  selectedDirectMessagePeerPubkey?: string | null;
  selectedThread?: string | null;
  settingsOpen?: boolean;
  settingsSection?: SettingsSection;
  timelineScope?: TimelineScope;
  timelineView?: TimelineWorkspaceView;
  selectedChannelId?: string | null;
};

export type OpenThreadOptions = {
  historyMode?: 'push' | 'replace';
  normalizeOnEmpty?: boolean;
  topic?: string;
};

export type OpenAuthorOptions = {
  fromThread?: boolean;
  historyMode?: 'push' | 'replace';
  normalizeOnError?: boolean;
  threadId?: string | null;
  preserveDirectMessageContext?: boolean;
  directMessagePeerPubkey?: string | null;
};

export const PRIMARY_SECTION_ITEMS: Array<{
  id: PrimarySection;
  label: string;
}> = [
  { id: 'timeline', label: 'Timeline' },
  { id: 'live', label: 'Live' },
  { id: 'game', label: 'Game' },
  { id: 'messages', label: 'Messages' },
  { id: 'profile', label: 'Profile' },
];

export const SETTINGS_SECTION_COPY: Array<{
  id: SettingsSection;
  label: string;
  description: string;
}> = [
  {
    id: 'appearance',
    label: 'Appearance',
    description: 'Local light and dark theme selection.',
  },
  {
    id: 'connectivity',
    label: 'Connectivity',
    description: 'Sync summary, peer tickets, and global error visibility.',
  },
  {
    id: 'discovery',
    label: 'Discovery',
    description: 'Seeded DHT configuration and discovery diagnostics.',
  },
  {
    id: 'community-node',
    label: 'Community Node',
    description: 'Configured community nodes, auth, consent, and refresh actions.',
  },
  {
    id: 'reactions',
    label: 'Reactions',
    description: 'Custom reaction creation and saved reaction management.',
  },
];

export const PRIMARY_SECTION_PATHS: Record<PrimarySection, string> = {
  timeline: '/timeline',
  live: '/live',
  game: '/game',
  messages: '/messages',
  profile: '/profile',
};

export function isSettingsSection(value: string | null): value is SettingsSection {
  return (
    value === 'appearance' ||
    value === 'connectivity' ||
    value === 'discovery' ||
    value === 'community-node' ||
    value === 'reactions'
  );
}

export function isProfileConnectionsView(
  value: string | null
): value is ProfileConnectionsView {
  return value === 'following' || value === 'followed' || value === 'muted';
}

export function parsePrimarySectionPath(pathname: string): PrimarySection | null {
  const normalizedPath = pathname === '/' ? '/timeline' : pathname;
  if (normalizedPath === '/channels') {
    return null;
  }
  const match = (
    Object.entries(PRIMARY_SECTION_PATHS) as Array<[PrimarySection, string]>
  ).find(([, path]) => path === normalizedPath);
  return match?.[0] ?? null;
}

export function parseLegacyRequestedChannel(
  requestedTimelineScopeValue: string | null,
  requestedComposeTargetValue: string | null
): string | null {
  return [requestedComposeTargetValue, requestedTimelineScopeValue]
    .filter((value): value is string => Boolean(value))
    .map((value) => {
      if (value.startsWith('channel:')) {
        return value.slice('channel:'.length);
      }
      return null;
    })
    .find((value): value is string => value !== null) ?? null;
}

export type BuildShellUrlOptions = {
  activeTopic: string;
  primarySection: PrimarySection;
  timelineView: TimelineWorkspaceView;
  profileMode: ProfileWorkspaceMode;
  profileConnectionsView: ProfileConnectionsView;
  selectedThread: string | null;
  selectedAuthorPubkey: string | null;
  selectedDirectMessagePeerPubkey: string | null;
  settingsOpen: boolean;
  settingsSection: SettingsSection;
  selectedChannelId: string | null;
};

export function buildShellUrl(options: BuildShellUrlOptions): string {
  const search = new URLSearchParams();
  search.set('topic', options.activeTopic);

  if (
    options.primarySection !== 'messages' &&
    options.selectedChannelId &&
    !(options.primarySection === 'timeline' && options.timelineView === 'bookmarks')
  ) {
    search.set('channel', options.selectedChannelId);
  }

  if (options.primarySection === 'timeline' && options.timelineView === 'bookmarks') {
    search.set('timelineView', 'bookmarks');
  }

  if (options.primarySection === 'messages') {
    if (options.selectedDirectMessagePeerPubkey) {
      search.set('peerPubkey', options.selectedDirectMessagePeerPubkey);
    }
    if (options.selectedAuthorPubkey) {
      search.set('authorPubkey', options.selectedAuthorPubkey);
    }
  } else if (options.selectedThread) {
    search.set('context', 'thread');
    search.set('threadId', options.selectedThread);
    if (options.selectedAuthorPubkey) {
      search.set('authorPubkey', options.selectedAuthorPubkey);
    }
  } else if (options.selectedAuthorPubkey) {
    search.set('context', 'author');
    search.set('authorPubkey', options.selectedAuthorPubkey);
  }

  if (options.primarySection === 'profile' && options.profileMode === 'edit') {
    search.set('profileMode', 'edit');
  }
  if (options.primarySection === 'profile' && options.profileMode === 'connections') {
    search.set('profileMode', 'connections');
    search.set('connectionsView', options.profileConnectionsView);
  }
  if (options.settingsOpen) {
    search.set('settings', options.settingsSection);
  }

  const nextPath = PRIMARY_SECTION_PATHS[options.primarySection];
  const nextSearch = search.toString();
  return nextSearch ? `${nextPath}?${nextSearch}` : nextPath;
}
