import { describe, it, expect, beforeEach } from 'vitest';
import { persistKeys } from '@/stores/config/persist';
import { resolveThemeFromStorage, useUIStore } from '@/stores/uiStore';

describe('uiStore', () => {
  beforeEach(() => {
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      timelineUpdateMode: 'standard',
      isLoading: false,
      error: null,
      activeSidebarCategory: null,
    });
  });

  it('初期状態が正しく設定されていること', () => {
    const state = useUIStore.getState();
    expect(state.sidebarOpen).toBe(true);
    expect(state.theme).toBe('system');
    expect(state.timelineUpdateMode).toBe('standard');
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.activeSidebarCategory).toBeNull();
  });

  it('toggleSidebarメソッドが正しく動作すること', () => {
    const { toggleSidebar } = useUIStore.getState();
    toggleSidebar();
    expect(useUIStore.getState().sidebarOpen).toBe(false);

    toggleSidebar();
    expect(useUIStore.getState().sidebarOpen).toBe(true);
  });

  it('setSidebarOpenメソッドが正しく動作すること', () => {
    const { setSidebarOpen } = useUIStore.getState();
    setSidebarOpen(false);
    expect(useUIStore.getState().sidebarOpen).toBe(false);

    setSidebarOpen(true);
    expect(useUIStore.getState().sidebarOpen).toBe(true);
  });

  it('setThemeメソッドが正しく動作すること', () => {
    const { setTheme } = useUIStore.getState();
    setTheme('dark');
    expect(useUIStore.getState().theme).toBe('dark');

    setTheme('light');
    expect(useUIStore.getState().theme).toBe('light');

    setTheme('system');
    expect(useUIStore.getState().theme).toBe('system');
  });

  it('setTimelineUpdateModeメソッドが正しく動作すること', () => {
    const { setTimelineUpdateMode } = useUIStore.getState();
    setTimelineUpdateMode('realtime');
    expect(useUIStore.getState().timelineUpdateMode).toBe('realtime');

    setTimelineUpdateMode('standard');
    expect(useUIStore.getState().timelineUpdateMode).toBe('standard');
  });

  it('setLoadingメソッドが正しく動作すること', () => {
    const { setLoading } = useUIStore.getState();
    setLoading(true);
    expect(useUIStore.getState().isLoading).toBe(true);

    setLoading(false);
    expect(useUIStore.getState().isLoading).toBe(false);
  });

  it('setErrorメソッドが正しく動作すること', () => {
    const errorMessage = 'エラーが発生しました';
    const { setError } = useUIStore.getState();
    setError(errorMessage);
    expect(useUIStore.getState().error).toBe(errorMessage);

    setError(null);
    expect(useUIStore.getState().error).toBeNull();
  });

  it('clearErrorメソッドが正しく動作すること', () => {
    useUIStore.setState({ error: 'テストエラー' });

    const { clearError } = useUIStore.getState();
    clearError();
    expect(useUIStore.getState().error).toBeNull();
  });

  it('setActiveSidebarCategoryメソッドが正しく動作すること', () => {
    const { setActiveSidebarCategory } = useUIStore.getState();
    setActiveSidebarCategory('trending');
    expect(useUIStore.getState().activeSidebarCategory).toBe('trending');

    // 同じ値の場合は更新されない
    const previousState = useUIStore.getState();
    setActiveSidebarCategory('trending');
    expect(useUIStore.getState()).toBe(previousState);
  });

  it('resetActiveSidebarCategoryメソッドが正しく動作すること', () => {
    useUIStore.setState({ activeSidebarCategory: 'search' });
    const { resetActiveSidebarCategory } = useUIStore.getState();
    resetActiveSidebarCategory();
    expect(useUIStore.getState().activeSidebarCategory).toBeNull();
  });

  it('永続化ストレージのuiキーからテーマを復元できる', () => {
    const storage: Pick<Storage, 'getItem'> = {
      getItem: (key: string) =>
        key === persistKeys.ui ? JSON.stringify({ state: { theme: 'dark' }, version: 0 }) : null,
    };

    expect(resolveThemeFromStorage(storage)).toBe('dark');
  });

  it('互換キーからテーマを復元できる', () => {
    const storage: Pick<Storage, 'getItem'> = {
      getItem: (key: string) => {
        if (key === persistKeys.ui) return null;
        if (key === 'kukuri-theme') return 'light';
        if (key === 'theme') return null;
        return null;
      },
    };

    expect(resolveThemeFromStorage(storage)).toBe('light');
  });

  it('不正な永続化値は復元対象外として扱う', () => {
    const storage: Pick<Storage, 'getItem'> = {
      getItem: () => 'not-a-theme',
    };

    expect(resolveThemeFromStorage(storage)).toBeNull();
  });
});
