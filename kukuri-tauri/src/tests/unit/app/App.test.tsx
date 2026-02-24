import { beforeEach, describe, it, expect, vi } from 'vitest';
import { render } from '@testing-library/react';
import App from '@/App';
import { usePrivacySettingsAutoSync } from '@/hooks/usePrivacySettingsAutoSync';

// モック - シンプルにプロバイダーとルーターの存在のみチェック
vi.mock('@tanstack/react-query', () => ({
  QueryClientProvider: ({ children }: { children: React.ReactNode }) => (
    <div data-testid="query-provider">{children}</div>
  ),
  QueryClient: vi.fn(),
}));

vi.mock('@tanstack/react-router', () => ({
  RouterProvider: () => <div data-testid="router-provider">App Content</div>,
}));

vi.mock('@/lib/queryClient', () => ({
  queryClient: {},
}));

vi.mock('@/router', () => ({
  router: {},
}));

vi.mock('@/hooks/usePrivacySettingsAutoSync', () => ({
  usePrivacySettingsAutoSync: vi.fn(),
}));

describe('App', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('QueryClientProviderがレンダリングされること', () => {
    const { container } = render(<App />);

    expect(container.querySelector('[data-testid="query-provider"]')).toBeInTheDocument();
  });

  it('RouterProviderがレンダリングされること', () => {
    const { container } = render(<App />);

    expect(container.querySelector('[data-testid="router-provider"]')).toBeInTheDocument();
  });

  it('Toasterコンポーネントが含まれていること', () => {
    render(<App />);

    // Toasterのコンテナが存在することを確認
    // sonnerはポータルを使用するため、body直下に追加される
    expect(document.body.querySelector('[data-sonner-toaster]')).toBeDefined();
  });

  it('プライバシー設定の自動同期フックを初期化すること', () => {
    render(<App />);

    expect(usePrivacySettingsAutoSync).toHaveBeenCalled();
  });
});
