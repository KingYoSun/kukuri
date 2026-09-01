export type PrimarySection =
  | 'timeline'
  | 'explore'
  | 'live'
  | 'game'
  | 'messages'
  | 'profile'
  | 'notifications';
export type TimelineWorkspaceView = 'feed' | 'bookmarks';
export type ProfileConnectionsView = 'following' | 'followed' | 'muted';

export type SettingsSection =
  | 'about'
  | 'appearance'
  | 'safety'
  | 'connectivity'
  | 'discovery'
  | 'community-node'
  | 'reactions'
  | 'release'
  | 'developer';

export type ProfileWorkspaceMode = 'overview' | 'edit' | 'connections';

export type ShellChromeState = {
  activeSettingsSection: SettingsSection;
  profileMode: ProfileWorkspaceMode;
  profileConnectionsView: ProfileConnectionsView;
  settingsOpen: boolean;
};

export type ShellChromeProjection = ShellChromeState & {
  activePrimarySection: PrimarySection;
  timelineView: TimelineWorkspaceView;
};
