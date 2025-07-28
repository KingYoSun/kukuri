import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { TopicSelector } from './TopicSelector';
import { useTopicStore } from '@/stores/topicStore';
import type { Topic } from '@/stores';

// モックを設定
vi.mock('@/stores/topicStore');

// モックトピックデータ
const mockTopics: Topic[] = [
  {
    id: 'topic1',
    name: 'プログラミング',
    description: 'プログラミングに関する話題',
    createdAt: new Date(),
    memberCount: 10,
    postCount: 20,
    isActive: true,
    tags: ['tech', 'coding'],
  },
  {
    id: 'topic2',
    name: '雑談',
    description: '日常的な話題',
    createdAt: new Date(),
    memberCount: 5,
    postCount: 15,
    isActive: true,
    tags: ['general'],
  },
  {
    id: 'topic3',
    name: '音楽',
    description: '音楽について語ろう',
    createdAt: new Date(),
    memberCount: 8,
    postCount: 12,
    isActive: true,
    tags: ['music'],
  },
];

describe('TopicSelector', () => {
  const mockOnValueChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    
    // デフォルトのモック設定
    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map(mockTopics.map(t => [t.id, t])),
      joinedTopics: ['topic1', 'topic2'], // topic1とtopic2に参加している
    } as any);
  });

  it('コンポーネントが正しくレンダリングされる', () => {
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    expect(button).toBeInTheDocument();
    expect(button).toHaveTextContent('トピックを選択');
  });

  it('値が選択されている場合、トピック名が表示される', () => {
    render(<TopicSelector value="topic1" onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    expect(button).toHaveTextContent('プログラミング');
  });

  it('カスタムプレースホルダーが表示される', () => {
    render(
      <TopicSelector 
        onValueChange={mockOnValueChange} 
        placeholder="カスタムプレースホルダー"
      />
    );

    const button = screen.getByRole('combobox');
    expect(button).toHaveTextContent('カスタムプレースホルダー');
  });

  it('disabled状態でボタンが無効になる', () => {
    render(<TopicSelector onValueChange={mockOnValueChange} disabled />);

    const button = screen.getByRole('combobox');
    expect(button).toBeDisabled();
  });

  it('クリックするとドロップダウンが開く', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    // 検索入力が表示される
    expect(screen.getByPlaceholderText('トピックを検索...')).toBeInTheDocument();
  });

  it('参加しているトピックのみ表示される', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    // 参加しているトピックが表示される
    expect(await screen.findByText('プログラミング')).toBeInTheDocument();
    expect(await screen.findByText('雑談')).toBeInTheDocument();
    
    // 参加していないトピックは表示されない
    expect(screen.queryByText('音楽')).not.toBeInTheDocument();
  });

  it('トピックの説明が表示される', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    expect(await screen.findByText('プログラミングに関する話題')).toBeInTheDocument();
    expect(await screen.findByText('日常的な話題')).toBeInTheDocument();
  });

  it('参加しているトピックがない場合、メッセージが表示される', async () => {
    vi.mocked(useTopicStore).mockReturnValue({
      topics: new Map(mockTopics.map(t => [t.id, t])),
      joinedTopics: [], // どのトピックにも参加していない
    } as any);

    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    expect(await screen.findByText('参加しているトピックがありません')).toBeInTheDocument();
  });

  it('トピックを選択するとonValueChangeが呼ばれる', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    const topic = await screen.findByText('プログラミング');
    await user.click(topic);

    expect(mockOnValueChange).toHaveBeenCalledWith('topic1');
  });

  it('トピックを選択するとドロップダウンが閉じる', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    // ドロップダウンが開いていることを確認
    expect(screen.getByPlaceholderText('トピックを検索...')).toBeInTheDocument();

    const topic = await screen.findByText('プログラミング');
    await user.click(topic);

    // ドロップダウンが閉じていることを確認
    await waitFor(() => {
      expect(screen.queryByPlaceholderText('トピックを検索...')).not.toBeInTheDocument();
    });
  });

  it('検索機能が動作する', async () => {
    const user = userEvent.setup();
    render(<TopicSelector onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    const searchInput = screen.getByPlaceholderText('トピックを検索...');
    await user.type(searchInput, 'プログ');

    // 部分一致するトピックが表示される
    expect(await screen.findByText('プログラミング')).toBeInTheDocument();
    
    // 一致しないトピックは表示されない
    await waitFor(() => {
      expect(screen.queryByText('雑談')).not.toBeInTheDocument();
    });
  });

  it('選択されているトピックにチェックマークが表示される', async () => {
    const user = userEvent.setup();
    render(<TopicSelector value="topic1" onValueChange={mockOnValueChange} />);

    const button = screen.getByRole('combobox');
    await user.click(button);

    // topic1の項目を探す
    const topic1Item = screen.getByText('プログラミング').closest('[role="option"]');
    expect(topic1Item).toBeInTheDocument();

    // チェックマークが表示されている（opacity-100クラス）
    const checkIcon = topic1Item?.querySelector('.lucide-check');
    expect(checkIcon).toHaveClass('opacity-100');
  });
});