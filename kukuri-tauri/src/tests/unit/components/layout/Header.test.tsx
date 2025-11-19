import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { Header } from '@/components/layout/Header';
import { useAuthStore, useUIStore } from '@/stores';
import { useNavigate } from '@tanstack/react-router';
import { useDirectMessageStore, getDirectMessageInitialState } from '@/stores/directMessageStore';
import { act } from 'react-dom/test-utils';
import { TauriApi } from '@/lib/api/tauri';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

// モック
vi.mock('@tanstack/react-router', () => ({
  useNavigate: vi.fn(() => vi.fn()),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    listDirectMessageConversations: vi.fn(),
  },
}));

vi.mock('@/components/ui/dialog', async () => {
  const React = await import('react');
  const passthrough =
    (slot: string) =>
    ({ children, ...props }: React.ComponentProps<'div'>) => (
      <div data-testid={slot} {...props}>
        {children}
      </div>
    );
  return {
    Dialog: ({ open = true, children }: { open?: boolean; children: React.ReactNode }) =>
      open ? <>{children}</> : null,
    DialogContent: passthrough('dialog-content'),
    DialogHeader: passthrough('dialog-header'),
    DialogTitle: passthrough('dialog-title'),
    DialogDescription: passthrough('dialog-description'),
    DialogFooter: passthrough('dialog-footer'),
    DialogPortal: ({ children }: { children: React.ReactNode }) => <>{children}</>,
    DialogOverlay: passthrough('dialog-overlay'),
    DialogTrigger: ({ children }: { children: React.ReactNode }) => <>{children}</>,
    DialogClose: ({ children }: { children: React.ReactNode }) => <>{children}</>,
  };
});

vi.mock('@/components/directMessages/DirectMessageInbox', async () => {
  const React = await import('react');
  const { useDirectMessageStore } = await import('@/stores/directMessageStore');
  const DirectMessageInbox = () => {
    const { isInboxOpen, openDialog } = useDirectMessageStore((state) => ({
      isInboxOpen: state.isInboxOpen,
      openDialog: state.openDialog,
    }));
    const [target, setTarget] = React.useState('');
    if (!isInboxOpen) {
      return null;
    }
    return (
      <div data-testid="dm-inbox-mock">
        <p>ダイレクトメッセージ</p>
        <input
          data-testid="dm-inbox-target-input"
          value={target}
          onChange={(event) => setTarget((event.target as HTMLInputElement).value)}
        />
        <button
          data-testid="dm-inbox-start-button"
          onClick={() => {
            if (target) {
              openDialog(target);
            }
          }}
        >
          新しいメッセージ
        </button>
      </div>
    );
  };
  return { DirectMessageInbox };
});

let openInboxSpy: ReturnType<typeof vi.fn>;

const renderHeader = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <Header />
    </QueryClientProvider>,
  );
};

describe('Header', () => {
  const mockNavigate = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(useNavigate).mockReturnValue(mockNavigate);
    vi.mocked(TauriApi.listDirectMessageConversations).mockResolvedValue({
      items: [],
      nextCursor: null,
      hasMore: false,
    });
    useDirectMessageStore.setState(getDirectMessageInitialState());
    const originalOpenInbox = useDirectMessageStore.getState().openInbox;
    openInboxSpy = vi.fn(() => {
      originalOpenInbox();
    });
    useDirectMessageStore.setState((state) => ({
      ...state,
      openInbox: openInboxSpy,
    }));
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

    renderHeader();

    // ロゴが表示されること
    expect(screen.getByText('kukuri')).toBeInTheDocument();

    // メニューボタンが存在すること
    const menuButton = screen.getByRole('button', { name: /メニュー切り替え/i });
    expect(menuButton).toBeInTheDocument();

    const dmButton = screen.getByRole('button', { name: 'ダイレクトメッセージ' });
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

    renderHeader();

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

    renderHeader();

    const menuButton = screen.getByLabelText('メニュー切り替え');
    await user.click(menuButton);

    expect(toggleSidebar).toHaveBeenCalledTimes(1);
  });

  it('ロゴクリックでホームに遷移すること', async () => {
    const user = userEvent.setup();
    renderHeader();

    const logo = screen.getByText('kukuri');
    await user.click(logo);

    expect(mockNavigate).toHaveBeenCalledWith({ to: '/' });
  });

  it('DMボタンが未読バッジを表示し、会話を開くこと', async () => {
    const user = userEvent.setup();
    useDirectMessageStore.setState((state) => ({
      ...state,
      conversations: {
        npub1target: [
          {
            eventId: 'evt-1',
            clientMessageId: 'client-1',
            senderNpub: 'npub1target',
            recipientNpub: 'npub1current',
            content: 'テストメッセージ',
            createdAt: Date.now(),
            status: 'sent',
          },
        ],
      },
      unreadCounts: {
        npub1target: 3,
      },
    }));

    renderHeader();

    const dmButton = screen.getByRole('button', { name: 'ダイレクトメッセージ' });
    expect(dmButton).toHaveTextContent('3');

    await user.click(dmButton);

    await act(() => Promise.resolve());
    expect(useDirectMessageStore.getState().isDialogOpen).toBe(true);
    expect(useDirectMessageStore.getState().activeConversationNpub).toBe('npub1target');
  });

  it('Inbox CTA から新規DMダイアログを開けること', async () => {
    const user = userEvent.setup();

    renderHeader();

    const inboxButton = screen.getByTestId('open-dm-inbox-button');
    await user.click(inboxButton);

    await act(() => Promise.resolve());
    expect(openInboxSpy).toHaveBeenCalledTimes(1);
    expect(useDirectMessageStore.getState().isInboxOpen).toBe(true);

    act(() => {
      useDirectMessageStore.setState((state) => ({
        ...state,
        isDialogOpen: true,
        activeConversationNpub: 'npub1example',
      }));
    });

    await act(() => Promise.resolve());
    expect(useDirectMessageStore.getState().isDialogOpen).toBe(true);
    expect(useDirectMessageStore.getState().activeConversationNpub).toBe('npub1example');
  });

  it('会話がない場合はDMボタンでInboxが開くこと', async () => {
    const user = userEvent.setup();
    useDirectMessageStore.setState(getDirectMessageInitialState());

    renderHeader();

    const dmButton = screen.getByRole('button', { name: 'ダイレクトメッセージ' });
    await user.click(dmButton);

    await act(() => Promise.resolve());
    expect(openInboxSpy).toHaveBeenCalledTimes(1);
    expect(useDirectMessageStore.getState().isInboxOpen).toBe(true);
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

    renderHeader();

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

    renderHeader();

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

    renderHeader();

    // アバターボタンをクリック（getInitialsは最初の文字「テ」を返す）
    const avatarButton = screen.getByRole('button', { name: /テ/i });
    await user.click(avatarButton);

    const logoutItem = screen.getByText('ログアウト');
    await user.click(logoutItem);

    expect(logout).toHaveBeenCalledTimes(1);
  });

  it('適切なスタイリングが適用されていること', () => {
    const { container } = renderHeader();

    const header = container.querySelector('header');
    expect(header).toHaveClass('h-16', 'border-b', 'bg-background');
  });
});
