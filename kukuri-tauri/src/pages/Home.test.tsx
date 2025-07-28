import { render, screen, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi } from 'vitest';
import Home from './Home';
import type { Post as TauriPost } from '@/lib/api/tauri';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    getPosts: vi.fn(),
    likePost: vi.fn(),
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
  beforeEach(() => {
    vi.clearAllMocks();
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

  it('投稿が0件の場合はメッセージを表示する', async () => {
    const { TauriApi } = await import('@/lib/api/tauri');
    vi.mocked(TauriApi.getPosts).mockResolvedValue([]);

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
});
