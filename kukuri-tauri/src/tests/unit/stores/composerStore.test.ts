import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useComposerStore } from '@/stores/composerStore';

describe('composerStore', () => {
  beforeEach(() => {
    useComposerStore.getState().reset();
  });

  it('初期状態では閉じていること', () => {
    const state = useComposerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.topicId).toBeNull();
    expect(state.onSuccess).toBeNull();
  });

  it('openComposerでオプションを設定できること', () => {
    const { openComposer } = useComposerStore.getState();
    const onSuccess = vi.fn();

    openComposer({
      topicId: 'topic-1',
      replyTo: 'reply-1',
      quotedPost: 'quoted-1',
      onSuccess,
    });

    const state = useComposerStore.getState();
    expect(state.isOpen).toBe(true);
    expect(state.topicId).toBe('topic-1');
    expect(state.replyTo).toBe('reply-1');
    expect(state.quotedPost).toBe('quoted-1');
    expect(state.onSuccess).toBe(onSuccess);
  });

  it('closeComposerで初期状態に戻ること', () => {
    const { openComposer, closeComposer } = useComposerStore.getState();
    openComposer({ topicId: 'topic-1' });
    closeComposer();

    const state = useComposerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.topicId).toBeNull();
    expect(state.onSuccess).toBeNull();
  });

  it('completeでコールバックが呼び出されること', () => {
    const { openComposer, complete } = useComposerStore.getState();
    const onSuccess = vi.fn();

    openComposer({ topicId: 'topic-1', onSuccess });
    complete();

    expect(onSuccess).toHaveBeenCalledTimes(1);
    const state = useComposerStore.getState();
    expect(state.isOpen).toBe(false);
    expect(state.topicId).toBeNull();
  });
});
