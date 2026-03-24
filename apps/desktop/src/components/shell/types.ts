export type PrimarySection = 'timeline' | 'channels' | 'live' | 'game' | 'profile';

export type ContextPaneMode = 'thread' | 'author';

export type SettingsSection = 'connectivity' | 'discovery' | 'community-node';

export type ShellChromeState = {
  activePrimarySection: PrimarySection;
  activeContextPaneMode: ContextPaneMode;
  activeSettingsSection: SettingsSection;
  navOpen: boolean;
  contextOpen: boolean;
  settingsOpen: boolean;
};
