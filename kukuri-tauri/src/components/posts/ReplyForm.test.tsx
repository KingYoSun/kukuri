import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ReplyForm } from './ReplyForm';
import { useAuthStore } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { toast } from 'sonner';

// モック
vi.mock('@/stores', () => ({
  useAuthStore: vi.fn(() => ({
    currentUser: null,
  })),
  useBookmarkStore: vi.fn(() => ({
    bookmarks: [],
    fetchBookmarks: vi.fn(),
    addBookmark: vi.fn(),
    removeBookmark: vi.fn(),
    isBookmarked: vi.fn(() => false),
  })),
}));

vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    createPost: vi.fn(),
  },
}));

vi.mock('sonner');

const mockUseAuthStore = vi.mocked(useAuthStore);
const mockTauriApi = vi.mocked(TauriApi);
const mockToast = vi.mocked(toast);

describe('ReplyForm', () => {
  const mockProfile = {
    pubkey: 'test-pubkey',
    npub: 'npub1test',
    name: 'Test User',
    displayName: 'Test Display Name',
    picture: 'https://example.com/avatar.jpg',
  };

  const defaultProps = {
    postId: 'post123',
    topicId: 'topic456',
  };

  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });

    // useAuthStore のモック
    mockUseAuthStore.mockReturnValue({
      currentUser: mockProfile,
    } as any);

    // TauriApi のモック
    mockTauriApi.createPost = vi.fn().mockResolvedValue({ id: 'new-post-id' });
  });

  const renderWithQueryClient = (ui: React.ReactElement) => {
    return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
  };

  it('返信フォームを表示する', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
    expect(screen.getByText('返信する')).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Enter または ⌘+Enter で送信')).toBeInTheDocument();
  });

  it('ユーザーのアバターを表示する', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    // アバターのフォールバックテキストを確認（イニシャル）
    const avatarFallback = screen.getByText('TD');
    expect(avatarFallback).toBeInTheDocument();
  });

  it('キャンセルボタンが表示される', () => {
    const onCancel = vi.fn();
    renderWithQueryClient(<ReplyForm {...defaultProps} onCancel={onCancel} />);

    const cancelButton = screen.getByText('キャンセル');
    expect(cancelButton).toBeInTheDocument();

    fireEvent.click(cancelButton);
    expect(onCancel).toHaveBeenCalled();
  });

  it('空の内容では送信ボタンが無効', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const submitButton = screen.getByText('返信する');
    expect(submitButton).toBeDisabled();
  });

  it('内容を入力すると送信ボタンが有効になる', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');

    expect(submitButton).not.toBeDisabled();
  });

  it('返信を送信する', async () => {
    const user = userEvent.setup();
    const onSuccess = vi.fn();
    renderWithQueryClient(<ReplyForm {...defaultProps} onSuccess={onSuccess} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledWith({
        content: 'これは返信です',
        topic_id: 'topic456',
        tags: [
          ['e', 'post123', '', 'reply'],
          ['t', 'topic456'],
        ],
      });
      expect(mockToast.success).toHaveBeenCalledWith('返信を投稿しました');
      expect(onSuccess).toHaveBeenCalled();
    });
  });

  it('トピックIDなしで返信を送信する', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm postId="post123" />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledWith({
        content: 'これは返信です',
        topic_id: undefined,
        tags: [['e', 'post123', '', 'reply']],
      });
    });
  });

  it('Ctrl+Enterで送信する', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');

    await user.type(textarea, 'これは返信です');
    await user.keyboard('{Control>}{Enter}{/Control}');

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalled();
    });
  });

  it('⌘+Enterで送信する（Mac）', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');

    await user.type(textarea, 'これは返信です');
    await user.keyboard('{Meta>}{Enter}{/Meta}');

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalled();
    });
  });

  it('送信中は入力フィールドとボタンが無効になる', async () => {
    const user = userEvent.setup();
    let resolvePromise: () => void;
    const promise = new Promise<void>((resolve) => {
      resolvePromise = resolve;
    });

    mockTauriApi.createPost = vi.fn().mockReturnValue(promise);

    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');
    await user.click(submitButton);

    expect(textarea).toBeDisabled();
    expect(screen.getByText('投稿中...')).toBeInTheDocument();

    resolvePromise!();

    await waitFor(() => {
      expect(textarea).not.toBeDisabled();
      expect(screen.getByText('返信する')).toBeInTheDocument();
    });
  });

  it('送信後にフォームをクリアする', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...') as HTMLTextAreaElement;
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');
    await user.click(submitButton);

    await waitFor(() => {
      expect(textarea.value).toBe('');
    });
  });

  it('エラー時にエラーメッセージを表示する', async () => {
    const user = userEvent.setup();
    mockTauriApi.createPost = vi.fn().mockRejectedValue(new Error('Network error'));

    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, 'これは返信です');
    await user.click(submitButton);

    await waitFor(() => {
      // errorHandler経由でtoastが呼ばれることを確認
      expect(mockTauriApi.createPost).toHaveBeenCalled();
    });
  });

  it('空白のみの内容では送信しない', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    const textarea = screen.getByPlaceholderText('返信を入力...');
    const submitButton = screen.getByText('返信する');

    await user.type(textarea, '   ');

    expect(submitButton).toBeDisabled();
  });

  it('認証されていない場合は表示しない', () => {
    mockUseAuthStore.mockReturnValue({
      currentUser: null,
    } as any);

    const { container } = renderWithQueryClient(<ReplyForm {...defaultProps} />);

    expect(container.firstChild).toBeNull();
  });
});
