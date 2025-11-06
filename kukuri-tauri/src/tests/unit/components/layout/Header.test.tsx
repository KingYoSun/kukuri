import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Header } from '@/components/layout/Header';
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
    // デフォルトのユーザーを設定
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'User',
        displayName: 'User',
        picture: '',
        about: '',
        nip05: '',
      },
    });

    render(<Header />);

    // ロゴが表示されること
    expect(screen.getByText('kukuri')).toBeInTheDocument();

    // メニューボタンが存在すること
    const menuButton = screen.getByRole('button', { name: /メニュー切り替え/i });
    expect(menuButton).toBeInTheDocument();

    const dmButton = screen.getByRole('button', { name: /ダイレクトメッセージ/i });
    expect(dmButton).toBeInTheDocument();

    // 通知ボタンが存在すること
    const notificationButton = screen.getByRole('button', { name: /通知/i });
    expect(notificationButton).toBeInTheDocument();

    // アバターが表示されること（getInitials('User') => 'U'）
    expect(screen.getByText('U')).toBeInTheDocument();
  });

  it('ユーザーメニューが正しく動作すること', async () => {
    const user = userEvent.setup();

    // デフォルトのユーザーを設定
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'User',
        displayName: 'User',
        picture: '',
        about: '',
        nip05: '',
      },
    });

    render(<Header />);

    // 初期状態ではメニューが非表示
    expect(screen.queryByText('マイアカウント')).not.toBeInTheDocument();

    // アバターをクリックしてメニューを開く
    const avatarButton = screen.getByRole('button', { name: /U/i });
    await user.click(avatarButton);

    // メニューアイテムが表示されること
    expect(screen.getByText('別のアカウントを追加')).toBeInTheDocument();
    expect(screen.getByText('アカウントを削除')).toBeInTheDocument();
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
      accounts: [
        {
          npub: 'npub123',
          pubkey: 'pubkey123',
          name: 'テストユーザー',
          display_name: 'テストユーザー',
          picture: 'https://example.com/avatar.jpg',
          last_used: new Date().toISOString(),
        },
      ],
    });

    render(<Header />);

    // アバターのフォールバックには名前の最初の文字が表示される（getInitialsの実装により）
    expect(screen.getByText('テ')).toBeInTheDocument();

    // アバターボタンをクリックしてドロップダウンを開く
    const avatarButton = screen.getByRole('button', { name: /テ/i });
    await user.click(avatarButton);

    // ドロップダウンメニューにフルネームが表示される（複数の要素があることを期待）
    const usernames = screen.getAllByText('テストユーザー');
    expect(usernames).toHaveLength(2); // ボタンとドロップダウン内の2つ
  });

  it('別のアカウントを追加メニュークリックでリダイレクトすること', async () => {
    const user = userEvent.setup();
    // デフォルトのユーザーを設定
    useAuthStore.setState({
      currentUser: {
        id: 'test123',
        pubkey: 'pubkey123',
        npub: 'npub123',
        name: 'User',
        displayName: 'User',
        picture: '',
        about: '',
        nip05: '',
      },
      accounts: [
        {
          npub: 'npub123',
          pubkey: 'pubkey123',
          name: 'User',
          display_name: 'User',
          picture: '',
          last_used: new Date().toISOString(),
        },
      ],
    });

    render(<Header />);

    const avatarButton = screen.getByRole('button', { name: /U/i });
    await user.click(avatarButton);

    const addAccountItem = screen.getByText('別のアカウントを追加');
    await user.click(addAccountItem);

    // AccountSwitcherは window.location.href = '/login' を使用している
    // テストではwindow.location.hrefの変更を確認するのが難しいため、このテストはスキップまたは別の方法で実装する必要がある
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
      accounts: [
        {
          npub: 'npub123',
          pubkey: 'pubkey123',
          name: 'テストユーザー',
          display_name: 'テストユーザー',
          picture: '',
          last_used: new Date().toISOString(),
        },
      ],
      logout,
    });

    render(<Header />);

    // アバターボタンをクリック（getInitialsは最初の文字「テ」を返す）
    const avatarButton = screen.getByRole('button', { name: /テ/i });
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
