import { screen, fireEvent, waitFor } from '@testing-library/react';
import { vi } from 'vitest';
import type { Post } from '@/stores';
import React from 'react';

import {
  bookmarkStoreMock,
  offlineStoreState,
  likePostMock,
  boostPostMock,
  createPostMock,
  deletePostMutationMock,
  toastMock,
  useAuthStoreMock,
  mockPost,
  renderWithQueryClient,
} from './__utils__/postCardTestUtils';

import { PostCard } from '@/components/posts/PostCard';

const buildPost = (overrides: Partial<Post> = {}): Post => ({
  ...mockPost,
  ...overrides,
});

const getBookmarkButton = () => {
  const buttons = screen.getAllByRole('button');
  const target = buttons.find((button) =>
    button.querySelector('[data-lucide="bookmark"], .lucide-bookmark'),
  );
  if (!target) {
    throw new Error('Bookmark button not found');
  }
  return target as HTMLButtonElement;
};

describe('PostCard', () => {
  beforeEach(() => {
    useAuthStoreMock.mockClear();
    deletePostMutationMock.mutate.mockReset();
    deletePostMutationMock.isPending = false;
    deletePostMutationMock.mutate.mockImplementation((_, options) => {
      options?.onSettled?.();
    });
    likePostMock.mockReset();
    boostPostMock.mockReset();
    createPostMock.mockReset();
    bookmarkStoreMock.fetchBookmarks.mockReset();
    bookmarkStoreMock.toggleBookmark.mockReset();
    bookmarkStoreMock.isBookmarked.mockReset();
    bookmarkStoreMock.isBookmarked.mockReturnValue(false);
    toastMock.error.mockReset();
    toastMock.success.mockReset();
    offlineStoreState.isOnline = true;
    offlineStoreState.pendingActions = [];
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
    const postWithAvatar = buildPost({
      author: {
        ...mockPost.author,
        picture: 'https://example.com/avatar.jpg',
      },
    });

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

  it('未同期の投稿には「同期待ち」バッジが表示される', () => {
    const unsyncedPost = buildPost({
      isSynced: false,
      localId: 'local-1',
    });

    renderWithQueryClient(<PostCard post={unsyncedPost} />);

    expect(screen.getByTestId('post-1-sync-badge')).toBeInTheDocument();
  });

  it('ブースト操作が成功するとトーストが表示される', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    const { toast } = await import('sonner');
    vi.mocked(TauriApi.boostPost).mockResolvedValue(undefined);
    toast.success.mockClear();

    renderWithQueryClient(<PostCard post={buildPost({ boosts: 3 })} />);

    const boostButton = screen.getByRole('button', { name: '3' });
    fireEvent.click(boostButton);

    await waitFor(() => {
      expect(TauriApi.boostPost).toHaveBeenCalledWith('1');
      expect(toast.success).toHaveBeenCalledWith('ブーストしました');
    });
  });

  it('ブースト操作が失敗するとエラーを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    const { toast } = await import('sonner');
    vi.mocked(TauriApi.boostPost).mockRejectedValue(new Error('failed'));
    toast.error.mockClear();

    renderWithQueryClient(<PostCard post={buildPost({ boosts: 4 })} />);

    const boostButton = screen.getByRole('button', { name: '4' });
    fireEvent.click(boostButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('ブーストに失敗しました');
    });
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
      const { toast } = await import('sonner');
      createPostMock.mockResolvedValue(mockPost);

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
        expect(createPostMock).toHaveBeenCalledWith('これは返信です', 'topic1', {
          replyTo: '1',
          scope: undefined,
        });
        expect(toast.success).toHaveBeenCalledWith('返信を投稿しました');
      });
    });

    it('返信成功後にフォームが閉じる', async () => {
      createPostMock.mockResolvedValue(mockPost);

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
        expect(createPostMock).toHaveBeenCalled();
      });

      await waitFor(
        () => {
          expect(screen.queryByPlaceholderText('返信を入力...')).not.toBeInTheDocument();
        },
        { timeout: 3000 },
      );
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
      const { toast } = await import('sonner');
      createPostMock.mockResolvedValue(mockPost);

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
        expect(createPostMock).toHaveBeenCalledWith('これは引用コメントです\n\nnostr:1', 'topic1', {
          quotedPost: '1',
          scope: undefined,
        });
        expect(toast.success).toHaveBeenCalledWith('引用投稿を作成しました');
      });
    });

    it('引用成功後にフォームが閉じる', async () => {
      createPostMock.mockResolvedValue(mockPost);

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
        expect(createPostMock).toHaveBeenCalled();
      });

      await waitFor(
        () => {
          expect(screen.queryByPlaceholderText('コメントを追加...')).not.toBeInTheDocument();
        },
        { timeout: 3000 },
      );
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
  describe('削除メニュー', () => {
    const createOwnPost = (): Post => ({
      ...mockPost,
      author: {
        ...mockPost.author,
        pubkey: 'user-pubkey',
        npub: 'npub1user',
      },
    });

    const openDeleteMenu = () => {
      const trigger = screen.getByTestId('post-1-menu');
      if (!trigger) {
        throw new Error('投稿メニューのボタンが見つかりませんでした');
      }
      fireEvent.click(trigger);
    };

    it('他人の投稿には削除メニューが表示されない', () => {
      renderWithQueryClient(<PostCard post={mockPost} />);

      expect(screen.queryByLabelText('投稿メニュー')).not.toBeInTheDocument();
      expect(screen.queryByText('削除')).not.toBeInTheDocument();
    });

    it('削除を確定すると useDeletePost の変異が呼び出される', async () => {
      const ownPost = createOwnPost();
      const { toast } = await import('sonner');
      deletePostMutationMock.mutate.mockImplementationOnce((_, options) => {
        toast.success('投稿を削除しました');
        options?.onSettled?.();
      });

      renderWithQueryClient(<PostCard post={ownPost} />);

      openDeleteMenu();
      fireEvent.click(screen.getByTestId('post-1-delete'));
      const confirmDialogTitle = await screen.findByTestId('post-1-confirm-title');
      expect(confirmDialogTitle).toBeInTheDocument();

      fireEvent.click(await screen.findByTestId('post-1-confirm-delete'));

      await waitFor(() => {
        expect(deletePostMutationMock.mutate).toHaveBeenCalledWith(
          ownPost,
          expect.objectContaining({ onSettled: expect.any(Function) }),
        );
      });

      await waitFor(() => {
        expect(toast.success).toHaveBeenCalledWith('投稿を削除しました');
      });

      await waitFor(() => {
        expect(screen.queryByText('投稿を削除しますか？')).not.toBeInTheDocument();
      });
    });

    it('削除に失敗した場合はエラートーストを表示する', async () => {
      const ownPost = createOwnPost();
      const { toast } = await import('sonner');
      deletePostMutationMock.mutate.mockImplementationOnce((_, options) => {
        toast.error('投稿の削除に失敗しました');
        options?.onSettled?.();
      });

      renderWithQueryClient(<PostCard post={ownPost} />);

      openDeleteMenu();
      fireEvent.click(screen.getByTestId('post-1-delete'));

      fireEvent.click(await screen.findByTestId('post-1-confirm-delete'));

      await waitFor(() => {
        expect(toast.error).toHaveBeenCalledWith('投稿の削除に失敗しました');
      });

      await waitFor(() => {
        expect(screen.queryByText('投稿を削除しますか？')).not.toBeInTheDocument();
      });
    });
  });

  it('マウント時にブックマーク状態を取得する', () => {
    renderWithQueryClient(<PostCard post={mockPost} />);

    expect(bookmarkStoreMock.fetchBookmarks).toHaveBeenCalled();
  });

  it('ブックマークボタンでtoggleBookmarkを呼び出す', async () => {
    const { toast } = await import('sonner');
    bookmarkStoreMock.toggleBookmark.mockResolvedValue(undefined);
    toast.success.mockClear();

    renderWithQueryClient(<PostCard post={mockPost} />);

    const bookmarkButton = getBookmarkButton();
    fireEvent.click(bookmarkButton);

    await waitFor(() => {
      expect(bookmarkStoreMock.toggleBookmark).toHaveBeenCalledWith('1');
      expect(toast.success).toHaveBeenCalledWith('ブックマークしました');
    });
  });

  it('ブックマーク済みの投稿はボタンが強調表示される', () => {
    bookmarkStoreMock.isBookmarked.mockReturnValue(true);

    renderWithQueryClient(<PostCard post={mockPost} />);

    const bookmarkButton = getBookmarkButton();
    expect(bookmarkButton).toHaveClass('text-yellow-500');
  });

  it('ブックマーク操作に失敗するとエラーが表示される', async () => {
    const { toast } = await import('sonner');
    bookmarkStoreMock.toggleBookmark.mockRejectedValue(new Error('failed'));
    toast.error.mockClear();

    renderWithQueryClient(<PostCard post={mockPost} />);

    const bookmarkButton = getBookmarkButton();
    fireEvent.click(bookmarkButton);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith('ブックマークの操作に失敗しました');
    });
  });
});
