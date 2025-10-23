import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi } from 'vitest';
import Home from '@/pages/Home';
import type { Post as TauriPost } from '@/lib/api/tauri';
import { useTopicStore } from '@/stores/topicStore';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getPosts: vi.fn(),
    likePost: vi.fn(),
    createPost: vi.fn(),
  },
}));

// Mock PostCard component
vi.mock('@/components/posts/PostCard', () => ({
  PostCard: ({ post }: { post: TauriPost }) => (
    <div data-testid="post-card">
      <div>{post.content}</div>
    </div>
  ),
}));

// Mock PostComposer component
vi.mock('@/components/posts/PostComposer', () => ({
  PostComposer: ({ onSuccess, onCancel }: { onSuccess?: () => void; onCancel?: () => void }) => (
    <div data-testid="post-composer">
      <button
        data-testid="post-composer-submit"
        onClick={() => {
          onSuccess?.();
        }}
      >
        投稿する
      </button>
      <button data-testid="post-composer-cancel" onClick={() => onCancel?.()}>
        キャンセル
      </button>
    </div>
  ),
}));

// Mock topic store
vi.mock('@/stores/topicStore');

const mockPosts: TauriPost[] = [
  {
    id: '1',
    content: 'テスト投稿1',
    author_pubkey: 'pubkey1',
    topic_id: 'topic1',
    created_at: Math.floor(Date.now() / 1000),
    likes: 5,
    replies: 0,
  },
  {
    id: '2',
    content: 'テスト投稿2',
    author_pubkey: 'pubkey2',
    topic_id: 'topic2',
    created_at: Math.floor(Date.now() / 1000) - 3600,
    likes: 10,
    replies: 3,
  },
];

const renderWithQueryClient = (ui: React.ReactElement) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });

  return render(<QueryClientProvider client={queryClient}>{ui}</QueryClientProvider>);
};

describe('Home', () => {
  const mockUseTopicStore = vi.mocked(useTopicStore);

  beforeEach(() => {
    vi.clearAllMocks();
    // デフォルトのトピック状態
    mockUseTopicStore.mockReturnValue({
      joinedTopics: [],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);
  });

  it('タイトルを表示する', () => {
    renderWithQueryClient(<Home />);
    expect(screen.getByText('タイムライン')).toBeInTheDocument();
  });

  it('読み込み中の状態を表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockImplementation(() => new Promise(() => {})); // Never resolves

    renderWithQueryClient(<Home />);

    expect(screen.getByTestId('loader')).toBeInTheDocument();
  });

  it('投稿を表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      const postCards = screen.getAllByTestId('post-card');
      expect(postCards).toHaveLength(2);
    });

    expect(screen.getByText('テスト投稿1')).toBeInTheDocument();
    expect(screen.getByText('テスト投稿2')).toBeInTheDocument();
  });

  it('投稿が0件でトピックに参加していない場合は適切なメッセージを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue([]);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.getByText('トピックに参加すると、投稿が表示されます。')).toBeInTheDocument();
    });
  });

  it('投稿が0件でトピックに参加している場合は投稿を促すメッセージを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue([]);
    mockUseTopicStore.mockReturnValue({
      joinedTopics: ['topic1'],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(
        screen.getByText('まだ投稿がありません。最初の投稿をしてみましょう！'),
      ).toBeInTheDocument();
    });
  });

  it('エラーが発生した場合はエラーメッセージを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockRejectedValue(new Error('Failed to fetch'));

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(
        screen.getByText('投稿の取得に失敗しました。リロードしてください。'),
      ).toBeInTheDocument();
    });
  });

  it('50件を上限として投稿を取得する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(TauriApi.getPosts).toHaveBeenCalledWith({ limit: 50 });
    });
  });

  it('トピックに参加している場合は投稿ボタンを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);
    mockUseTopicStore.mockReturnValue({
      joinedTopics: ['topic1'],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
    });
  });

  it('トピックに参加していない場合は投稿ボタンを表示しない', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.queryByRole('button', { name: /投稿する/i })).not.toBeInTheDocument();
    });
  });

  it('投稿ボタンをクリックすると投稿フォームが表示される', async () => {
    const user = userEvent.setup();
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);
    mockUseTopicStore.mockReturnValue({
      joinedTopics: ['topic1'],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /投稿する/i }));

    expect(screen.getByTestId('post-composer')).toBeInTheDocument();
  });

  it('投稿フォームのキャンセルボタンをクリックするとフォームが閉じる', async () => {
    const user = userEvent.setup();
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);
    mockUseTopicStore.mockReturnValue({
      joinedTopics: ['topic1'],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
    });

    await user.click(screen.getByRole('button', { name: /投稿する/i }));
    expect(screen.getByTestId('post-composer')).toBeInTheDocument();

    await user.click(screen.getByTestId('post-composer-cancel'));
    expect(screen.queryByTestId('post-composer')).not.toBeInTheDocument();
  });

  it('投稿が成功するとフォームが閉じて投稿ボタンが再度表示される', async () => {
    const user = userEvent.setup();
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue(mockPosts);
    vi.mocked(TauriApi.createPost).mockResolvedValue({
      id: '3',
      content: '新しい投稿',
      author_pubkey: 'pubkey3',
      topic_id: 'topic1',
      created_at: Math.floor(Date.now() / 1000),
      likes: 0,
      replies: 0,
    });
    mockUseTopicStore.mockReturnValue({
      joinedTopics: ['topic1'],
    } as Partial<ReturnType<typeof useTopicStore>> as ReturnType<typeof useTopicStore>);

    renderWithQueryClient(<Home />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
    });

    // 投稿ボタンをクリックしてフォームを表示
    await user.click(screen.getByRole('button', { name: /投稿する/i }));
    expect(screen.getByTestId('post-composer')).toBeInTheDocument();

    // PostComposerのモック内で定義した「投稿する」ボタンをクリック
    const submitButton = screen.getByTestId('post-composer-submit');

    // クリックイベントを実行
    await user.click(submitButton);

    // フォームが閉じることを確認
    await waitFor(() => {
      expect(screen.queryByTestId('post-composer')).not.toBeInTheDocument();
    });

    // 投稿ボタンが再表示されることを確認
    expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
  });
});
