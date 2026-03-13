import { beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { GlobalComposer } from '@/components/posts/GlobalComposer';
import { useComposerStore } from '@/stores/composerStore';

vi.mock('@/components/posts/PostComposer', () => ({
  PostComposer: ({ onSuccess, onCancel }: { onSuccess?: () => void; onCancel?: () => void }) => (
    <div data-testid="mock-post-composer">
      <button data-testid="mock-post-composer-submit" onClick={onSuccess}>
        投稿する
      </button>
      <button data-testid="mock-post-composer-cancel" onClick={onCancel}>
        キャンセル
      </button>
    </div>
  ),
}));

describe('GlobalComposer', () => {
  beforeEach(() => {
    useComposerStore.getState().reset();
  });

  it('ストアが閉じているときは描画されない', () => {
    render(<GlobalComposer />);
    expect(screen.queryByTestId('mock-post-composer')).not.toBeInTheDocument();
  });

  it('ストアが開いているときにPostComposerを表示する', async () => {
    const user = userEvent.setup();
    useComposerStore.getState().openComposer({ topicId: 'topic-1' });

    render(<GlobalComposer />);

    expect(screen.getByTestId('mock-post-composer')).toBeInTheDocument();
    await user.click(screen.getByTestId('mock-post-composer-cancel'));
    expect(useComposerStore.getState().isOpen).toBe(false);
  });

  it('completeが呼ばれるとストアが閉じ、コールバックが実行される', async () => {
    const user = userEvent.setup();
    const onSuccess = vi.fn();
    useComposerStore.getState().openComposer({ onSuccess });

    render(<GlobalComposer />);

    await user.click(screen.getByTestId('mock-post-composer-submit'));

    expect(useComposerStore.getState().isOpen).toBe(false);
    expect(onSuccess).toHaveBeenCalledTimes(1);
  });
});
