import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';

import { TopicFormModal } from '@/components/topics/TopicFormModal';

const mockQueueTopicCreation = vi.fn();
const mockCreateTopic = vi.fn();
const mockJoinTopic = vi.fn();
const mockWatchPendingTopic = vi.fn();
const mockToast = vi.fn();

vi.mock('@/stores/topicStore', () => ({
  useTopicStore: () => ({
    createTopic: mockCreateTopic,
    queueTopicCreation: mockQueueTopicCreation,
    updateTopicRemote: vi.fn(),
    joinTopic: mockJoinTopic,
  }),
}));

vi.mock('@/stores/offlineStore', () => ({
  useOfflineStore: (selector: (state: { isOnline: boolean }) => boolean) =>
    selector({ isOnline: false }),
}));

vi.mock('@/stores/composerStore', () => ({
  useComposerStore: (
    selector: (state: { watchPendingTopic: typeof mockWatchPendingTopic }) => unknown,
  ) =>
    selector({
      watchPendingTopic: mockWatchPendingTopic,
    }),
}));

vi.mock('@/hooks/use-toast', () => ({
  useToast: () => ({ toast: mockToast }),
}));

describe('topicCreateOffline scenario', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockQueueTopicCreation.mockResolvedValue({
      pending_id: 'pending-123',
      name: 'オフライン作成',
      description: 'desc',
      status: 'queued',
      offline_action_id: 'action-1',
      synced_topic_id: null,
      error_message: null,
      created_at: Date.now() / 1000,
      updated_at: Date.now() / 1000,
    });
  });

  it('queues topic creation when offline and registers composer watcher', async () => {
    render(
      <TopicFormModal
        open
        onOpenChange={() => undefined}
        mode="create-from-composer"
        onCreated={vi.fn()}
      />,
    );

    await userEvent.type(screen.getByLabelText('トピック名 *'), 'オフライン作成');
    await userEvent.type(
      screen.getByPlaceholderText('このトピックに関する説明を入力してください'),
      'desc',
    );

    await userEvent.click(screen.getByRole('button', { name: '作成' }));

    await waitFor(() => {
      expect(mockQueueTopicCreation).toHaveBeenCalledWith('オフライン作成', 'desc');
    });
    expect(mockCreateTopic).not.toHaveBeenCalled();
    expect(mockWatchPendingTopic).toHaveBeenCalledWith('pending-123');
    expect(mockToast).toHaveBeenCalledWith(
      expect.objectContaining({
        title: '作成をキューに追加しました',
      }),
    );
  });
});
