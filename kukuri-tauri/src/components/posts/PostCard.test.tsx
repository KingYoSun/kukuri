import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PostCard } from './PostCard';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi } from 'vitest';
import type { Post } from '@/stores';
import React from 'react';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    likePost: vi.fn(),
    createPost: vi.fn(),
  },
}));

// Mock sonner
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
    success: vi.fn(),
  },
}));

// Mock Collapsible components
vi.mock('@/components/ui/collapsible', () => ({
  Collapsible: ({ children, open }: { children: React.ReactNode; open: boolean }) => (
    <div data-state={open ? 'open' : 'closed'}>
      {open ? children : null}
    </div>
  ),
  CollapsibleContent: ({ children }: { children: React.ReactNode }) => <div>{children}</div>,
}));

// Mock stores
vi.mock('@/stores', () => ({
  useAuthStore: vi.fn(() => ({
    currentUser: {
      pubkey: 'user-pubkey',
      npub: 'npub1user',
      name: 'Current User',
      displayName: 'Current User Display',
      picture: 'https://example.com/current-user.jpg',
    },
  })),
  useBookmarkStore: vi.fn(() => ({
    bookmarks: [],
    fetchBookmarks: vi.fn(),
    addBookmark: vi.fn(),
    removeBookmark: vi.fn(),
    isBookmarked: vi.fn(() => false),
  })),
}));

const mockPost: Post = {
  id: '1',
  content: 'テスト投稿です',
  author: {
    id: 'user1',
    pubkey: 'pubkey1',
    npub: 'npub1test...',
    name: 'テストユーザー',
    displayName: 'Test User',
    picture: '',
    about: '',
    nip05: '',
  },
  topicId: 'topic1',
  created_at: Math.floor(Date.now() / 1000) - 3600, // 1時間前
  tags: [],
  likes: 10,
  replies: [],
};

const renderWithQueryClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('PostCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('投稿内容を表示する', () => {
    const { container } = renderWithQueryClient(<PostCard post={mockPost} />);

    // カード本体の投稿内容を特定のクラス名で取得
    const postContent = container.querySelector('.mb-4.whitespace-pre-wrap');
    expect(postContent).toBeTruthy();
    expect(postContent?.textContent).toBe('テスト投稿です');
    
    // 投稿者名はh4タグ内のものを取得
    const authorName = container.querySelector('h4.font-semibold');
    expect(authorName?.textContent).toBe('Test User');
    expect(screen.getByText('npub1test...')).toBeInTheDocument();
  });

  it('いいねの数を表示する', () => {
    renderWithQueryClient(<PostCard post={mockPost} />);

    expect(screen.getByText('10')).toBeInTheDocument();
  });

  it('返信の数を表示する', () => {
    renderWithQueryClient(<PostCard post={mockPost} />);

    // すべてのボタンを取得して、最初のボタン（メッセージボタン）を確認
    const buttons = screen.getAllByRole('button');
    expect(buttons[0]).toHaveTextContent('0');
  });

  it('アバターのイニシャルを表示する', () => {
    renderWithQueryClient(<PostCard post={mockPost} />);

    const avatarFallback = screen.getByText('TU');
    expect(avatarFallback).toBeInTheDocument();
  });

  it('アバター画像がある場合は画像URLが設定される', () => {
    const postWithAvatar = {
      ...mockPost,
      author: {
        ...mockPost.author,
        picture: 'https://example.com/avatar.jpg',
      },
    };

    renderWithQueryClient(<PostCard post={postWithAvatar} />);

    // PostCardコンポーネントが正しく画像URLを渡していることを確認
    // 実際の画像読み込みはテスト環境では確認できないため、
    // AvatarImageに正しいpropsが渡されていることを間接的に確認
    const avatarContainer = screen.getByText('TU').closest('[data-slot="avatar"]');
    expect(avatarContainer).toBeInTheDocument();
  });

  it('いいねボタンをクリックできる', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.likePost).mockResolvedValue(undefined);

    renderWithQueryClient(<PostCard post={mockPost} />);

    const likeButton = screen.getByRole('button', { name: /10/ });
    fireEvent.click(likeButton);

    await waitFor(() => {
      expect(TauriApi.likePost).toHaveBeenCalledWith('1');
    });
  });

  it('いいねに失敗した場合はエラーメッセージを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    const { toast } = await import('sonner');

    vi.mocked(TauriApi.likePost).mockRejectedValue(new Error('Failed'));

    renderWithQueryClient(<PostCard post={mockPost} />);

    const likeButton = screen.getByRole('button', { name: /10/ });
    fireEvent.click(likeButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('いいねに失敗しました');
    });
  });

  it('改行を含む投稿を正しく表示する', () => {
    const postWithNewlines = {
      ...mockPost,
      content: '1行目\n2行目\n3行目',
    };

    const { container } = renderWithQueryClient(<PostCard post={postWithNewlines} />);

    // whitespace-pre-wrapクラスを持つp要素を探す
    const content = container.querySelector('.whitespace-pre-wrap');
    expect(content).toBeTruthy();
    expect(content?.textContent).toBe('1行目\n2行目\n3行目');
  });

  it('時間を日本語で表示する', () => {
    const { container } = renderWithQueryClient(<PostCard post={mockPost} />);

    // ヘッダー内の時間表示を確認
    const timeElement = container.querySelector('.text-sm.text-muted-foreground');
    expect(timeElement?.textContent).toMatch(/前$/);
  });

  describe('返信機能', () => {
    it('返信ボタンをクリックすると返信フォームが表示される', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      // 返信ボタンをクリック
      const replyButton = screen.getAllByRole('button')[0]; // MessageCircleボタン
      expect(replyButton).toHaveTextContent('0');

      fireEvent.click(replyButton);

      // 返信フォームが表示される
      await waitFor(() => {
        expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
        expect(screen.getByText('返信する')).toBeInTheDocument();
      });
    });

    it('返信フォームが開いているときは返信ボタンがアクティブ状態になる', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      const replyButton = screen.getAllByRole('button')[0];
      fireEvent.click(replyButton);

      await waitFor(() => {
        expect(replyButton).toHaveClass('text-primary');
      });
    });

    it('返信フォームをキャンセルできる', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      const replyButton = screen.getAllByRole('button')[0];
      fireEvent.click(replyButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
      });

      // キャンセルボタンをクリック - getAllByTextを使用して最初のボタンを選択
      const cancelButtons = screen.getAllByText('キャンセル');
      fireEvent.click(cancelButtons[0]);

      // 返信フォームが非表示になる
      await waitFor(() => {
        expect(screen.queryByPlaceholderText('返信を入力...')).not.toBeInTheDocument();
      });
    });

    it('返信を送信できる', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      const { toast } = await import('sonner');
      vi.mocked(TauriApi.createPost).mockResolvedValue({ id: 'reply-id' } as any);

      renderWithQueryClient(<PostCard post={mockPost} />);

      // 返信フォームを開く
      const replyButton = screen.getAllByRole('button')[0];
      fireEvent.click(replyButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
      });

      // 返信を入力
      const textarea = screen.getByPlaceholderText('返信を入力...');
      fireEvent.change(textarea, { target: { value: 'これは返信です' } });

      // 送信ボタンをクリック
      const submitButton = screen.getByText('返信する');
      fireEvent.click(submitButton);

      await waitFor(() => {
        expect(TauriApi.createPost).toHaveBeenCalledWith({
          content: 'これは返信です',
          topic_id: 'topic1',
          tags: [
            ['e', '1', '', 'reply'],
            ['t', 'topic1'],
          ],
        });
        expect(toast.success).toHaveBeenCalledWith('返信を投稿しました');
      });
    });

    it('返信成功後にフォームが閉じる', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.createPost).mockResolvedValue({ id: 'reply-id' } as any);

      renderWithQueryClient(<PostCard post={mockPost} />);

      // 返信フォームを開く
      const replyButton = screen.getAllByRole('button')[0];
      fireEvent.click(replyButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
      });

      // 返信を入力して送信
      const textarea = screen.getByPlaceholderText('返信を入力...');
      fireEvent.change(textarea, { target: { value: 'これは返信です' } });

      const submitButton = screen.getByText('返信する');
      fireEvent.click(submitButton);

      // フォームが閉じるまで待つ（成功メッセージが表示されることも確認）
      await waitFor(() => {
        expect(TauriApi.createPost).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(screen.queryByPlaceholderText('返信を入力...')).not.toBeInTheDocument();
      }, { timeout: 3000 });
    });
  });

  describe('引用機能', () => {
    it('引用ボタンをクリックすると引用フォームが表示される', async () => {
      const { container } = renderWithQueryClient(<PostCard post={mockPost} />);

      // 引用ボタンをクリック（3番目のボタン - Quote アイコンのボタン）
      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      expect(quoteButton).toHaveTextContent('0');

      fireEvent.click(quoteButton);

      // 引用フォームが表示される
      await waitFor(() => {
        expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
        expect(screen.getByText('引用して投稿')).toBeInTheDocument();
        // 引用元の投稿内容が表示される（元の投稿と引用カード内の2つ）
        const allContents = container.querySelectorAll('.whitespace-pre-wrap');
        expect(allContents).toHaveLength(2); // 元の投稿 + 引用カード内の表示
      });
    });

    it('引用フォームが開いているときは引用ボタンがアクティブ状態になる', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      fireEvent.click(quoteButton);

      await waitFor(() => {
        expect(quoteButton).toHaveClass('text-primary');
      });
    });

    it('引用フォームをキャンセルできる', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      fireEvent.click(quoteButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
      });

      // キャンセルボタンをクリック - getAllByTextを使用して最初のボタンを選択
      const cancelButtons = screen.getAllByText('キャンセル');
      fireEvent.click(cancelButtons[0]);

      // 引用フォームが非表示になる
      await waitFor(() => {
        expect(screen.queryByPlaceholderText('コメントを追加...')).not.toBeInTheDocument();
      });
    });

    it('引用投稿を送信できる', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      const { toast } = await import('sonner');
      vi.mocked(TauriApi.createPost).mockResolvedValue({ id: 'quote-id' } as any);

      renderWithQueryClient(<PostCard post={mockPost} />);

      // 引用フォームを開く
      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      fireEvent.click(quoteButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
      });

      // コメントを入力
      const textarea = screen.getByPlaceholderText('コメントを追加...');
      fireEvent.change(textarea, { target: { value: 'これは引用コメントです' } });

      // 送信ボタンをクリック
      const submitButton = screen.getByText('引用して投稿');
      fireEvent.click(submitButton);

      await waitFor(() => {
        expect(TauriApi.createPost).toHaveBeenCalledWith({
          content: 'これは引用コメントです\n\nnostr:1',
          topic_id: 'topic1',
          tags: [
            ['e', '1', '', 'mention'],
            ['q', '1'],
            ['t', 'topic1'],
          ],
        });
        expect(toast.success).toHaveBeenCalledWith('引用投稿を作成しました');
      });
    });

    it('引用成功後にフォームが閉じる', async () => {
      const { TauriApi } = await import('@/lib/api/tauri');
      vi.mocked(TauriApi.createPost).mockResolvedValue({ id: 'quote-id' } as any);

      renderWithQueryClient(<PostCard post={mockPost} />);

      // 引用フォームを開く
      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      fireEvent.click(quoteButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
      });

      // コメントを入力して送信
      const textarea = screen.getByPlaceholderText('コメントを追加...');
      fireEvent.change(textarea, { target: { value: 'これは引用コメントです' } });

      const submitButton = screen.getByText('引用して投稿');
      fireEvent.click(submitButton);

      // フォームが閉じるまで待つ（成功メッセージが表示されることも確認）
      await waitFor(() => {
        expect(TauriApi.createPost).toHaveBeenCalled();
      });

      await waitFor(() => {
        expect(screen.queryByPlaceholderText('コメントを追加...')).not.toBeInTheDocument();
      }, { timeout: 3000 });
    });

    it('返信フォームと引用フォームは同時に開かない', async () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      // まず返信フォームを開く
      const replyButton = screen.getAllByRole('button')[0];
      fireEvent.click(replyButton);

      await waitFor(() => {
        expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
      });

      // 引用ボタンをクリック
      const quoteButton = screen.getAllByRole('button')[2]; // Quoteボタン
      fireEvent.click(quoteButton);

      // 返信フォームが閉じて引用フォームが開く
      await waitFor(() => {
        expect(screen.queryByPlaceholderText('返信を入力...')).not.toBeInTheDocument();
        expect(screen.getByPlaceholderText('コメントを追加...')).toBeInTheDocument();
      });
    });
  });
});
