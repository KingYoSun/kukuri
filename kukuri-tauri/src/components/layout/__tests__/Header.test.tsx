import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Header } from '../Header';
import { useAuthStore, useUIStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';

// モック
vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(() => vi.fn()),
}));

describe('Header', () => {
  const mockNavigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);
  });
  it('ヘッダーの基本要素が表示されること', () => {
    render(<Header />);

    // ロゴが表示されること
    expect(screen.getByText('kukuri')).toBeInTheDocument();

    // メニューボタンが存在すること
    const menuButton = screen.getByRole('button', { name: /メニュー切り替え/i });
    expect(menuButton).toBeInTheDocument();

    // 通知ボタンが存在すること
    const notificationButton = screen.getByRole('button', { name: /通知/i });
    expect(notificationButton).toBeInTheDocument();

    // アバターが表示されること
    expect(screen.getByText('U')).toBeInTheDocument();
  });

  it('ユーザーメニューが正しく動作すること', async () => {
    const user = userEvent.setup();
    render(<Header />);

    // 初期状態ではメニューが非表示
    expect(screen.queryByText('マイアカウント')).not.toBeInTheDocument();

    // アバターをクリックしてメニューを開く
    const avatarButton = screen.getByRole('button', { name: /U/i });
    await user.click(avatarButton);

    // メニューアイテムが表示されること
    expect(screen.getByText('マイアカウント')).toBeInTheDocument();
    expect(screen.getByText('設定')).toBeInTheDocument();
    expect(screen.getByText('ログアウト')).toBeInTheDocument();
  });

  it('メニューボタンクリックでサイドバーがトグルされること', async () => {
    const user = userEvent.setup();
    const toggleSidebar = vi.fn();
    useUIStore.setState({ toggleSidebar });

    render(<Header />);

    const menuButton = screen.getByLabelText('メニュー切り替え');
    await user.click(menuButton);

    expect(toggleSidebar).toHaveBeenCalledTimes(1);
  });

  it('ロゴクリックでホームに遷移すること', async () => {
    const user = userEvent.setup();
    render(<Header />);

    const logo = screen.getByText('kukuri');
    await user.click(logo);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('ユーザー情報が表示されること', async () => {
    const user = userEvent.setup();
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'テストユーザー',
        displayName: 'テストユーザー',
        picture: 'https://example.com/avatar.jpg',
        about: '',
        nip05: '',
      },
    });

    render(<Header />);

    // アバターのフォールバックには最初の2文字が表示される
    expect(screen.getByText('テス')).toBeInTheDocument();

    // アバターボタンをクリックしてドロップダウンを開く
    const avatarButton = screen.getByRole('button', { name: /テス/i });
    await user.click(avatarButton);

    // ドロップダウンメニューにフルネームが表示される
    expect(screen.getByText('テストユーザー')).toBeInTheDocument();
  });

  it('設定メニュークリックで設定ページに遷移すること', async () => {
    const user = userEvent.setup();
    render(<Header />);

    const avatarButton = screen.getByRole('button', { name: /U/i });
    await user.click(avatarButton);

    const settingsItem = screen.getByText('設定');
    await user.click(settingsItem);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/settings' });
  });

  it('ログアウトボタンクリックでログアウトされること', async () => {
    const user = userEvent.setup();
    const logout = vi.fn();
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'テストユーザー',
        displayName: 'テストユーザー',
        picture: '',
        about: '',
        nip05: '',
      },
      logout,
    });

    render(<Header />);

    // アバターボタンをクリック（最初の2文字「テス」で検索）
    const avatarButton = screen.getByRole('button', { name: /テス/i });
    await user.click(avatarButton);

    const logoutItem = screen.getByText('ログアウト');
    await user.click(logoutItem);

    expect(logout).toHaveBeenCalledTimes(1);
  });

  it('適切なスタイリングが適用されていること', () => {
    const { container } = render(<Header />);

    const header = container.querySelector('header');
    expect(header).toHaveClass('h-16', 'border-b', 'bg-background');
  });
});
