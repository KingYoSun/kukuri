import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactionPicker } from '../ReactionPicker';
import { NostrAPI } from '@/lib/api/tauri';
import { toast } from 'sonner';

vi.mock('@/lib/api/tauri');
vi.mock('sonner');

const mockNostrAPI = NostrAPI as unknown as {
  sendReaction: ReturnType<typeof vi.fn>;
};

const mockToast = toast as unknown as {
  success: ReturnType<typeof vi.fn>;
  error: ReturnType<typeof vi.fn>;
};

describe('ReactionPicker', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    queryClient = new QueryClient({
      defaultOptions: {
        queries: { retry: false },
        mutations: { retry: false },
      },
    });
    vi.clearAllMocks();
  });

  const renderReactionPicker = (postId = 'post1', topicId = 'topic1') => {
    return render(
      <QueryClientProvider client={queryClient}>
        <ReactionPicker postId={postId} topicId={topicId} />
      </QueryClientProvider>
    );
  };

  it('should render reaction picker button', () => {
    renderReactionPicker();
    const button = screen.getByRole('button');
    expect(button).toBeInTheDocument();
  });

  it('should open popover when clicked', () => {
    renderReactionPicker();
    const button = screen.getByRole('button');
    
    fireEvent.click(button);
    
    // ポピュラーなリアクションが表示されることを確認
    expect(screen.getByText('👍')).toBeInTheDocument();
    expect(screen.getByText('❤️')).toBeInTheDocument();
    expect(screen.getByText('😄')).toBeInTheDocument();
  });

  it('should send reaction when emoji is clicked', async () => {
    mockNostrAPI.sendReaction = vi.fn().mockResolvedValue('event123');
    mockToast.success = vi.fn();

    renderReactionPicker();
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const reactionButton = screen.getByText('👍');
    fireEvent.click(reactionButton);

    await waitFor(() => {
      expect(mockNostrAPI.sendReaction).toHaveBeenCalledWith('post1', '👍');
      expect(mockToast.success).toHaveBeenCalledWith('リアクションを送信しました');
    });
  });

  it('should handle reaction error', async () => {
    mockNostrAPI.sendReaction = vi.fn().mockRejectedValue(new Error('Failed'));
    mockToast.error = vi.fn();

    renderReactionPicker();
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const reactionButton = screen.getByText('👍');
    fireEvent.click(reactionButton);

    await waitFor(() => {
      expect(mockNostrAPI.sendReaction).toHaveBeenCalledWith('post1', '👍');
      expect(mockToast.error).toHaveBeenCalledWith('リアクションの送信に失敗しました');
    });
  });

  it('should close popover after successful reaction', async () => {
    mockNostrAPI.sendReaction = vi.fn().mockResolvedValue('event123');
    mockToast.success = vi.fn();

    renderReactionPicker();
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const reactionButton = screen.getByText('👍');
    fireEvent.click(reactionButton);

    await waitFor(() => {
      expect(mockToast.success).toHaveBeenCalled();
    });

    // ポップオーバーが閉じられたことを確認
    expect(screen.queryByText('😄')).not.toBeInTheDocument();
  });

  it('should disable button while sending reaction', async () => {
    mockNostrAPI.sendReaction = vi.fn().mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve('event123'), 100))
    );

    renderReactionPicker();
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const reactionButton = screen.getByText('👍');
    fireEvent.click(reactionButton);

    // ボタンが無効になることを確認
    expect(button).toBeDisabled();

    await waitFor(() => {
      expect(button).not.toBeDisabled();
    });
  });

  it('should render all popular reactions', () => {
    renderReactionPicker();
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const expectedReactions = [
      '👍', '❤️', '😄', '😂', '😮', '😢', '😡', '🔥',
      '💯', '🎉', '🚀', '👀', '🤔', '👏', '💪', '🙏',
    ];

    expectedReactions.forEach((reaction) => {
      expect(screen.getByText(reaction)).toBeInTheDocument();
    });
  });

  it('should invalidate queries after successful reaction', async () => {
    mockNostrAPI.sendReaction = vi.fn().mockResolvedValue('event123');
    const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries');

    renderReactionPicker('post1', 'topic123');
    const button = screen.getByRole('button');
    fireEvent.click(button);

    const reactionButton = screen.getByText('👍');
    fireEvent.click(reactionButton);

    await waitFor(() => {
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['timeline'] });
      expect(invalidateQueriesSpy).toHaveBeenCalledWith({ queryKey: ['posts', 'topic123'] });
    });
  });
});