import { describe, it, expect, beforeEach } from 'vitest';
import { useUIStore } from '../uiStore';

describe('uiStore', () => {
  beforeEach(() => {
    useUIStore.setState({
      sidebarOpen: true,
      theme: 'system',
      isLoading: false,
      error: null,
    });
  });

  it('初期状態が正しく設定されていること', () => {
    const state = useUIStore.getState();
    expect(state.sidebarOpen).toBe(true);
    expect(state.theme).toBe('system');
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
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
});
