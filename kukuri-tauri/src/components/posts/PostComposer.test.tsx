import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { PostComposer } from './PostComposer';
import { usePostStore } from '@/stores/postStore';
import { useTopicStore } from '@/stores/topicStore';
import { useToast } from '@/hooks/use-toast';
import type { Topic } from '@/stores';

// モックを設定
vi.mock('@/stores/postStore');
vi.mock('@/stores/topicStore');
vi.mock('@/hooks/use-toast');

// モックトピックデータ
const mockTopics: Topic[] = [
  {
    id: 'topic1',
    name: 'テストトピック1',
    description: '説明1',
    createdAt: new Date(),
    memberCount: 5,
    postCount: 10,
    isActive: true,
    tags: [],
  },
  {
    id: 'topic2',
    name: 'テストトピック2',
    description: '説明2',
    createdAt: new Date(),
    memberCount: 3,
    postCount: 5,
    isActive: true,
    tags: [],
  },
];

describe('PostComposer', () => {
  const mockCreatePost = vi.fn();
  const mockOnSuccess = vi.fn();
  const mockOnCancel = vi.fn();
  const mockToast = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    
    // usePostStoreのモック
    vi.mocked(usePostStore).mockReturnValue({
      createPost: mockCreatePost,
    } as any);

    // useTopicStoreのモック
    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map(mockTopics.map(t => [t.id, t])),
      joinedTopics: ['topic1', 'topic2'],
    } as any);

    // useToastのモック
    vi.mocked(useToast).mockReturnValue({
      toast: mockToast,
    } as any);
  });

  it('コンポーネントが正しくレンダリングされる', () => {
    render(<PostComposer />);

    expect(screen.getByLabelText('トピック')).toBeInTheDocument();
    expect(screen.getByLabelText('投稿内容')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /投稿する/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /キャンセル/i })).toBeInTheDocument();
  });

  it('参加しているトピックが選択できる', async () => {
    const user = userEvent.setup();
    render(<PostComposer />);

    const selectTrigger = screen.getByRole('combobox');
    await user.click(selectTrigger);

    // トピックが表示されることを確認
    expect(await screen.findByText('テストトピック1')).toBeInTheDocument();
    expect(await screen.findByText('テストトピック2')).toBeInTheDocument();
  });

  it('参加しているトピックがない場合メッセージが表示される', () => {
    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map(),
      joinedTopics: [],
    } as any);

    render(<PostComposer />);

    const selectTrigger = screen.getByRole('combobox');
    userEvent.click(selectTrigger);

    waitFor(() => {
      expect(screen.getByText('参加しているトピックがありません')).toBeInTheDocument();
    });
  });

  it('固定トピックIDが渡された場合、トピック選択が無効になる', () => {
    render(<PostComposer topicId="topic1" />);

    const selectTrigger = screen.getByRole('combobox');
    expect(selectTrigger).toHaveAttribute('disabled');
  });

  it('投稿内容が空の場合、送信ボタンが無効になる', () => {
    render(<PostComposer topicId="topic1" />);

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    expect(submitButton).toBeDisabled();
  });

  it('投稿内容とトピックが選択されている場合、送信ボタンが有効になる', async () => {
    const user = userEvent.setup();
    render(<PostComposer />);

    // トピックを選択
    const selectTrigger = screen.getByRole('combobox');
    await user.click(selectTrigger);
    await user.click(await screen.findByText('テストトピック1'));

    // 投稿内容を入力
    const textarea = screen.getByPlaceholderText('今何を考えていますか？');
    await user.type(textarea, 'テスト投稿');

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    expect(submitButton).toBeEnabled();
  });

  it('投稿が成功した場合、成功メッセージが表示される', async () => {
    const user = userEvent.setup();
    mockCreatePost.mockResolvedValue({
      id: 'post1',
      content: 'テスト投稿',
      topicId: 'topic1',
    });

    render(<PostComposer topicId="topic1" onSuccess={mockOnSuccess} />);

    const textarea = screen.getByPlaceholderText('今何を考えていますか？');
    await user.type(textarea, 'テスト投稿');

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockCreatePost).toHaveBeenCalledWith('テスト投稿', 'topic1');
      expect(mockToast).toHaveBeenCalledWith({
        title: '成功',
        description: '投稿を作成しました',
      });
      expect(mockOnSuccess).toHaveBeenCalled();
    });
  });

  it('投稿内容が空の場合、エラーメッセージが表示される', async () => {
    const user = userEvent.setup();
    render(<PostComposer topicId="topic1" />);

    const textarea = screen.getByPlaceholderText('今何を考えていますか？');
    await user.type(textarea, '   '); // 空白のみ

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith({
        title: 'エラー',
        description: '投稿内容を入力してください',
        variant: 'destructive',
      });
    });
  });

  it('トピックが選択されていない場合、エラーメッセージが表示される', async () => {
    const user = userEvent.setup();
    render(<PostComposer />);

    const textarea = screen.getByPlaceholderText('今何を考えていますか？');
    await user.type(textarea, 'テスト投稿');

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    await user.click(submitButton);

    await waitFor(() => {
      expect(mockToast).toHaveBeenCalledWith({
        title: 'エラー',
        description: 'トピックを選択してください',
        variant: 'destructive',
      });
    });
  });

  it('キャンセルボタンをクリックするとonCancelが呼ばれる', async () => {
    const user = userEvent.setup();
    render(<PostComposer onCancel={mockOnCancel} />);

    const cancelButton = screen.getByRole('button', { name: /キャンセル/i });
    await user.click(cancelButton);

    expect(mockOnCancel).toHaveBeenCalled();
  });

  it('送信中は入力フィールドとボタンが無効になる', async () => {
    const user = userEvent.setup();
    
    // 送信を遅延させる
    mockCreatePost.mockImplementation(() => new Promise(resolve => setTimeout(resolve, 100)));

    render(<PostComposer topicId="topic1" />);

    const textarea = screen.getByPlaceholderText('今何を考えていますか？');
    await user.type(textarea, 'テスト投稿');

    const submitButton = screen.getByRole('button', { name: /投稿する/i });
    await user.click(submitButton);

    // 送信中の状態を確認
    expect(textarea).toBeDisabled();
    expect(submitButton).toBeDisabled();
    expect(screen.getByRole('button', { name: /キャンセル/i })).toBeDisabled();
  });
});