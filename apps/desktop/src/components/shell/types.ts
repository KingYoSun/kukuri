export type PrimarySection = 'timeline' | 'live' | 'game' | 'profile';
export type TimelineWorkspaceView = 'feed' | 'bookmarks';
export type ProfileConnectionsView = 'following' | 'followed' | 'muted';

export type SettingsSection =
  | 'appearance'
  | 'connectivity'
  | 'discovery'
  | 'community-node'
  | 'reactions';

export type ProfileWorkspaceMode = 'overview' | 'edit' | 'connections';

export type ShellChromeState = {
  activePrimarySection: PrimarySection;
  timelineView: TimelineWorkspaceView;
  activeSettingsSection: SettingsSection;
  profileMode: ProfileWorkspaceMode;
  profileConnectionsView: ProfileConnectionsView;
  navOpen: boolean;
  settingsOpen: boolean;
};
