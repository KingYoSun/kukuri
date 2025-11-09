import { beforeEach, describe, expect, it } from 'vitest';

import { usePrivacySettingsStore } from '@/stores/privacySettingsStore';

describe('privacySettingsStore', () => {
  beforeEach(() => {
    usePrivacySettingsStore.getState().reset();
    localStorage.clear();
  });

  it('初期状態が正しいこと', () => {
    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(true);
    expect(state.showOnlineStatus).toBe(false);
  });

  it('公開設定を更新できること', () => {
    const { setPublicProfile } = usePrivacySettingsStore.getState();
    setPublicProfile(false);
    expect(usePrivacySettingsStore.getState().publicProfile).toBe(false);
  });

  it('オンライン表示設定を更新できること', () => {
    const { setShowOnlineStatus } = usePrivacySettingsStore.getState();
    setShowOnlineStatus(true);
    expect(usePrivacySettingsStore.getState().showOnlineStatus).toBe(true);
  });

  it('hydrateFromUserでユーザー設定を反映できること', () => {
    const { hydrateFromUser } = usePrivacySettingsStore.getState();
    hydrateFromUser({ publicProfile: false, showOnlineStatus: true });
    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
  });

  it('resetで初期状態に戻せること', () => {
    const { setPublicProfile, setShowOnlineStatus, reset } = usePrivacySettingsStore.getState();
    setPublicProfile(false);
    setShowOnlineStatus(true);

    reset();

    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(true);
    expect(state.showOnlineStatus).toBe(false);
  });
});
