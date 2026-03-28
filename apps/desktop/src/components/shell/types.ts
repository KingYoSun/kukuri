export type PrimarySection = 'timeline' | 'channels' | 'live' | 'game' | 'profile';

export type SettingsSection =
  | 'appearance'
  | 'connectivity'
  | 'discovery'
  | 'community-node'
  | 'reactions';

export type ProfileWorkspaceMode = 'overview' | 'edit';

export type ShellChromeState = {
  activePrimarySection: PrimarySection;
  activeSettingsSection: SettingsSection;
  profileMode: ProfileWorkspaceMode;
  navOpen: boolean;
  settingsOpen: boolean;
};
