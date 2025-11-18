import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { QuoteForm } from '@/components/posts/QuoteForm';
import type { Post } from '@/stores';
import {
  createMockProfile,
  createPostFormRenderer,
  mockTauriApi,
  mockToast,
  mockUseAuthStore,
} from './__utils__/postFormTestUtils';

describe('QuoteForm', () => {
  const mockProfile = createMockProfile();
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

  let renderWithQueryClient: ReturnType<typeof createPostFormRenderer>['renderWithQueryClient'];
  let resetQueryClient: ReturnType<typeof createPostFormRenderer>['reset'];

  beforeEach(() => {
    const helpers = createPostFormRenderer();
    renderWithQueryClient = helpers.renderWithQueryClient;
    resetQueryClient = helpers.reset;

    mockUseAuthStore.mockReturnValue({
      currentUser: mockProfile,
    } as never);

    mockTauriApi.createPost = vi.fn().mockResolvedValue({ id: 'new-quote-id' });
  });

  afterEach(() => {
    resetQueryClient?.();
  });

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

  it('キャンセルボタンが表示される', () => {
    const onCancel = vi.fn();
    renderWithQueryClient(<QuoteForm post={mockPost} onCancel={onCancel} />);

    const cancelButton = screen.getByText('キャンセル');
    fireEvent.click(cancelButton);
    expect(onCancel).toHaveBeenCalled();
  });

  it('空の内容では送信ボタンが無効', () => {
    renderWithQueryClient(<QuoteForm post={mockPost} />);
    expect(screen.getByText('引用して投稿')).toBeDisabled();
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

    await user.type(screen.getByPlaceholderText('コメントを追加...'), 'これは引用コメントです');
    await user.click(screen.getByText('引用して投稿'));

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
    const postWithoutTopic: Post = { ...mockPost, topicId: undefined };
    renderWithQueryClient(<QuoteForm post={postWithoutTopic} />);

    await user.type(screen.getByPlaceholderText('コメントを追加...'), 'これは引用コメントです');
    await user.click(screen.getByText('引用して投稿'));

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledWith({
        content: 'これは引用コメントです\n\nnostr:post123',
        topic_id: undefined,
        tags: [
          ['e', 'post123', '', 'mention'],
          ['q', 'post123'],
        ],
      });
    });
  });

  it('ショートカットで送信できる', async () => {
    const user = userEvent.setup();
    renderWithQueryClient(<QuoteForm post={mockPost} />);

    const textarea = screen.getByPlaceholderText('コメントを追加...');
    await user.type(textarea, 'これは引用コメントです');
    await user.keyboard('{Control>}{Enter}{/Control}');
    await user.keyboard('{Meta>}{Enter}{/Meta}');

    await waitFor(() => {
      expect(mockTauriApi.createPost).toHaveBeenCalledTimes(2);
    });
  });

  it('送信中の状態を表示する', async () => {
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
    expect(submitButton).toBeDisabled();

    resolvePromise!();
  });
});
