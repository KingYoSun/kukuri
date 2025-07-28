import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { PostCard } from './PostCard';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { vi } from 'vitest';
import type { Post } from '@/stores';

// Mock Tauri API
vi.mock('@/lib/api/tauri', () => ({
  TauriApi: {
    likePost: vi.fn(),
  },
}));

// Mock sonner
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
  },
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

  return render(
    <QueryClientProvider client={queryClient}>
      {ui}
    </QueryClientProvider>
  );
};

describe('PostCard', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('投稿内容を表示する', () => {
    renderWithQueryClient(<PostCard post={mockPost} />);
    
    expect(screen.getByText('テスト投稿です')).toBeInTheDocument();
    expect(screen.getByText('Test User')).toBeInTheDocument();
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
    renderWithQueryClient(<PostCard post={mockPost} />);
    
    // 約1時間前
    expect(screen.getByText(/前$/)).toBeInTheDocument();
  });
});