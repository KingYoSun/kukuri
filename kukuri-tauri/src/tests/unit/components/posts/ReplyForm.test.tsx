import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { ReplyForm } from '@/components/posts/ReplyForm';
import {
  createMockProfile,
  createPostFormRenderer,
  mockTauriApi,
  mockToast,
  mockUseAuthStore,
} from './__utils__/postFormTestUtils';

describe('ReplyForm', () => {
  const mockProfile = createMockProfile();
  const defaultProps = {
    postId: 'post123',
    topicId: 'topic456',
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

    mockTauriApi.createPost = vi.fn().mockResolvedValue({ id: 'new-post-id' });
  });

  afterEach(() => {
    resetQueryClient?.();
  });

  it('返信フォームを表示する', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);

    expect(screen.getByPlaceholderText('返信を入力...')).toBeInTheDocument();
    expect(screen.getByText('返信する')).toBeInTheDocument();
    expect(screen.getByText('Ctrl+Enter または ⌘+Enter で送信')).toBeInTheDocument();
  });

  it('ユーザーのアバターを表示する', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);
    expect(screen.getByText('TD')).toBeInTheDocument();
  });

  it('キャンセルボタンが表示される', () => {
    const onCancel = vi.fn();
    renderWithQueryClient(<ReplyForm {...defaultProps} onCancel={onCancel} />);

    const cancelButton = screen.getByText('キャンセル');
    fireEvent.click(cancelButton);
    expect(onCancel).toHaveBeenCalled();
  });

  it('空の内容では送信ボタンが無効', () => {
    renderWithQueryClient(<ReplyForm {...defaultProps} />);
    expect(screen.getByText('返信する')).toBeDisabled();
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

    await user.type(screen.getByPlaceholderText('返信を入力...'), 'これは返信です');
    await user.click(screen.getByText('返信する'));

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
    expect(submitButton).toBeDisabled();

    resolvePromise!();
  });
});
