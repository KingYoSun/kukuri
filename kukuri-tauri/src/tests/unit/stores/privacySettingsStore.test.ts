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
    expect(state.ownerNpub).toBeNull();
    expect(state.hasPendingSync).toBe(false);
    expect(state.lastSyncedAt).toBeNull();
    expect(state.lastSyncError).toBeNull();
    expect(state.updatedAt).toBeNull();
  });

  it('公開設定の更新で未同期フラグが立つこと', () => {
    const { setPublicProfile } = usePrivacySettingsStore.getState();
    setPublicProfile(false);
    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(false);
    expect(state.hasPendingSync).toBe(true);
    expect(state.updatedAt).not.toBeNull();
  });

  it('applyLocalChangeでローカル変更を保存できること', () => {
    const { applyLocalChange } = usePrivacySettingsStore.getState();
    applyLocalChange({
      npub: 'npub1alice',
      publicProfile: false,
      showOnlineStatus: true,
    });
    const state = usePrivacySettingsStore.getState();
    expect(state.ownerNpub).toBe('npub1alice');
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
    expect(state.hasPendingSync).toBe(true);
  });

  it('markSyncSuccessで同期済み状態に遷移できること', () => {
    const { applyLocalChange, markSyncSuccess } = usePrivacySettingsStore.getState();
    applyLocalChange({ npub: 'npub1alice', publicProfile: false });
    markSyncSuccess();
    const state = usePrivacySettingsStore.getState();
    expect(state.hasPendingSync).toBe(false);
    expect(state.lastSyncError).toBeNull();
    expect(state.lastSyncedAt).not.toBeNull();
  });

  it('hydrateFromUserで同一ユーザーかつ未同期時はローカル値を維持すること', () => {
    const { applyLocalChange, hydrateFromUser } = usePrivacySettingsStore.getState();
    applyLocalChange({ npub: 'npub1alice', publicProfile: false, showOnlineStatus: true });
    hydrateFromUser({
      npub: 'npub1alice',
      publicProfile: true,
      showOnlineStatus: false,
    });
    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
    expect(state.hasPendingSync).toBe(true);
  });

  it('hydrateFromUserで別ユーザーの値を反映できること', () => {
    const { hydrateFromUser } = usePrivacySettingsStore.getState();
    hydrateFromUser({
      npub: 'npub1bob',
      publicProfile: false,
      showOnlineStatus: true,
    });
    const state = usePrivacySettingsStore.getState();
    expect(state.ownerNpub).toBe('npub1bob');
    expect(state.publicProfile).toBe(false);
    expect(state.showOnlineStatus).toBe(true);
    expect(state.hasPendingSync).toBe(false);
  });

  it('resetで初期状態に戻せること', () => {
    const { applyLocalChange, reset } = usePrivacySettingsStore.getState();
    applyLocalChange({ npub: 'npub1alice', publicProfile: false, showOnlineStatus: true });

    reset();

    const state = usePrivacySettingsStore.getState();
    expect(state.publicProfile).toBe(true);
    expect(state.showOnlineStatus).toBe(false);
    expect(state.ownerNpub).toBeNull();
    expect(state.hasPendingSync).toBe(false);
    expect(state.lastSyncedAt).toBeNull();
  });
});
