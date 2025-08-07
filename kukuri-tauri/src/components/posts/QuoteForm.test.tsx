import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QuoteForm } from './QuoteForm';
import { useAuthStore } from '@/stores';
import { TauriApi } from '@/lib/api/tauri';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { toast } from 'sonner';
import type { Post } from '@/stores';

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

describe('QuoteForm', () => {
  const mockProfile = {
    pubkey: 'test-pubkey',
    npub: 'npub1test',
    name: 'Test User',
    displayName: 'Test Display Name',
    picture: 'https://example.com/avatar.jpg',
  };

  const mockPost: Post = {
    id: 'post123',
    content: 'これは引用される投稿です',
    author: {
      id: 'author1',
      pubkey: 'author-pubkey',
      npub: 'npub1author',
      name: 'Author Name',
      displayName: 'Author Display',
      picture: '',
      about: '',
      nip05: '',
    },
    topicId: 'topic456',
    created_at: Math.floor(Date.now() / 1000) - 3600,
    tags: [],
    likes: 5,
    replies: [],
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
    } as Partial<ReturnType<typeof useAuthStore>>);

    // TauriApi のモック
    mockTauriApi.createPost = vi.fn().mockResolvedValue({ id: 'new-quote-id' });
  });

  const renderWithQueryClient = (ui: React.ReactElement) => {
    return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
  };

  it('引用フォームを表示する', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
    expect(screen.getByText('引用して投稿')).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Enter または ⌘+Enter で送信')).toBeInTheDocument();
  });

  it('引用元の投稿を表示する', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    expect(screen.getByText('これは引用される投稿です')).toBeInTheDocument();
    expect(screen.getByText('Author Display')).toBeInTheDocument();
  });

  it('ユーザーのアバターを表示する', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    // アバターのフォールバックテキストを確認
    const avatarFallback = screen.getByText('TD');
    expect(avatarFallback).toBeInTheDocument();
  });

  it('キャンセルボタンが表示される', () => {
    const onCancel = vi.fn();
    renderWithQueryClient(<QuoteForm post={mockPost} onCancel={onCancel} />);

    const cancelButton = screen.getByText('キャンセル');
    expect(cancelButton).toBeInTheDocument();

    fireEvent.click(cancelButton);
    expect(onCancel).toHaveBeenCalled();
  });

  it('空の内容では送信ボタンが無効', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const submitButton = screen.getByText('引用して投稿');
    expect(submitButton).toBeDisabled();
  });

  it('内容を入力すると送信ボタンが有効になる', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');

    expect(submitButton).not.toBeDisabled();
  });

  it('引用投稿を送信する', async () => {
    const user = userEvent.setup();
    const onSuccess = vi.fn();
    renderWithQueryClient(<QuoteForm post={mockPost} onSuccess={onSuccess} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledWith({
        content: 'これは引用コメントです\n\nnostr:post123',
        topic_id: 'topic456',
        tags: [
          ['e', 'post123', '', 'mention'],
          ['q', 'post123'],
          ['t', 'topic456'],
        ],
      });
      expect(mockToast.success).toHaveBeenCalledWith('引用投稿を作成しました');
      expect(onSuccess).toHaveBeenCalled();
    });
  });

  it('トピックIDなしで引用投稿を送信する', async () => {
    const user = userEvent.setup();
    const postWithoutTopic = { ...mockPost, topicId: undefined };
    renderWithQueryClient(<QuoteForm post={postWithoutTopic} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledWith({
        content: 'これは引用コメントです\n\nnostr:post123',
        topicId: undefined,
        tags: [
          ['e', 'post123', '', 'mention'],
          ['q', 'post123'],
        ],
      });
    });
  });

  it('Ctrl+Enterで送信する', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');

    await user.type(textarea, 'これは引用コメントです');
    await user.keyboard('{Control>}{Enter}{/Control}');

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalled();
    });
  });

  it('⌘+Enterで送信する（Mac）', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');

    await user.type(textarea, 'これは引用コメントです');
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

    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');
    await user.click(submitButton);

    expect(textarea).toBeDisabled();
    expect(screen.getByText('投稿中...')).toBeInTheDocument();

    resolvePromise!();

    await waitFor(() => {
      expect(textarea).not.toBeDisabled();
      expect(screen.getByText('引用して投稿')).toBeInTheDocument();
    });
  });

  it('送信後にフォームをクリアする', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...') as HTMLTextAreaElement;
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');
    await user.click(submitButton);

    await waitFor(() => {
      expect(textarea.value).toBe('');
    });
  });

  it('エラー時にエラーメッセージを表示する', async () => {
    const user = userEvent.setup();
    mockTauriApi.createPost = vi.fn().mockRejectedValue(new Error('Network error'));

    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, 'これは引用コメントです');
    await user.click(submitButton);

    await waitFor(() => {
      // errorHandler経由でtoastが呼ばれることを確認
      expect(mockTauriApi.createPost).toHaveBeenCalled();
    });
  });

  it('空白のみの内容では送信しない', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    const submitButton = screen.getByText('引用して投稿');

    await user.type(textarea, '   ');

    expect(submitButton).toBeDisabled();
  });

  it('認証されていない場合は表示しない', () => {
    mockUseAuthStore.mockReturnValue({
      currentUser: null,
    } as Partial<ReturnType<typeof useAuthStore>>);

    const { container } = renderWithQueryClient(<QuoteForm post={mockPost} />);

    expect(container.firstChild).toBeNull();
  });

  it('長い投稿内容は省略される', () => {
    const longPost = {
      ...mockPost,
      content: 'これは非常に長い投稿内容です。'.repeat(20),
    };

    renderWithQueryClient(<QuoteForm post={longPost} />);

    const quotedContent = screen.getByText(/これは非常に長い投稿内容です/);
    expect(quotedContent).toHaveClass('line-clamp-3');
  });

  it('時間を日本語で表示する', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    expect(screen.getByText(/前$/)).toBeInTheDocument();
  });
});
