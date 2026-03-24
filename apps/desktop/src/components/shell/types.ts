export type PrimarySection = 'timeline' | 'channels' | 'live' | 'game';

export type ContextPaneMode = 'thread' | 'author';

export type SettingsSection = 'profile' | 'connectivity' | 'discovery' | 'community-node';

export type ShellChromeState = {
  activePrimarySection: PrimarySection;
  activeContextPaneMode: ContextPaneMode;
  activeSettingsSection: SettingsSection;
  navOpen: boolean;
  contextOpen: boolean;
  settingsOpen: boolean;
};
