import { create } from 'zustand';

import { errorHandler } from '@/lib/errorHandler';

interface ComposerOptions {
  topicId?: string | null;
  replyTo?: string | null;
  quotedPost?: string | null;
  onSuccess?: (() => void) | null;
}

interface ComposerState {
  isOpen: boolean;
  topicId: string | null;
  replyTo: string | null;
  quotedPost: string | null;
  onSuccess: (() => void) | null;
  pendingTopicBindings: Map<string, boolean>;
  openComposer: (options?: ComposerOptions) => void;
  closeComposer: () => void;
  complete: () => void;
  reset: () => void;
  applyTopicAndResume: (topicId: string) => void;
  watchPendingTopic: (pendingId: string) => void;
  resolvePendingTopic: (pendingId: string, topicId: string) => void;
  clearPendingTopicBinding: (pendingId: string) => void;
}

const createInitialState = (): Omit<
  ComposerState,
  'openComposer' | 'closeComposer' | 'complete' | 'reset' | 'applyTopicAndResume'
> => ({
  isOpen: false,
  topicId: null,
  replyTo: null,
  quotedPost: null,
  onSuccess: null,
  pendingTopicBindings: new Map(),
});

export const getComposerInitialState = () => createInitialState();

export const useComposerStore = create<ComposerState>((set, get) => ({
  ...createInitialState(),
  openComposer: (options = {}) =>
    set({
      isOpen: true,
      topicId: options.topicId ?? null,
      replyTo: options.replyTo ?? null,
      quotedPost: options.quotedPost ?? null,
      onSuccess: options.onSuccess ?? null,
    }),
  closeComposer: () => set(createInitialState()),
  complete: () => {
    const callback = get().onSuccess;
    if (callback) {
      try {
        callback();
      } catch (error) {
        errorHandler.log('Composer onSuccess callback failed', error, {
          context: 'ComposerStore.complete',
        });
      }
    }
    set(createInitialState());
  },
  reset: () => set(createInitialState()),
  applyTopicAndResume: (topicId: string) =>
    set((state) => ({
      ...state,
      topicId,
      isOpen: true,
    })),
  watchPendingTopic: (pendingId: string) =>
    set((state) => {
      const bindings = new Map(state.pendingTopicBindings);
      bindings.set(pendingId, true);
      return { pendingTopicBindings: bindings };
    }),
  resolvePendingTopic: (pendingId: string, topicId: string) => {
    const state = get();
    if (!state.pendingTopicBindings.has(pendingId)) {
      return;
    }
    set((current) => {
      const bindings = new Map(current.pendingTopicBindings);
      bindings.delete(pendingId);
      return { pendingTopicBindings: bindings };
    });
    state.applyTopicAndResume(topicId);
  },
  clearPendingTopicBinding: (pendingId: string) =>
    set((state) => {
      if (!state.pendingTopicBindings.has(pendingId)) {
        return state;
      }
      const bindings = new Map(state.pendingTopicBindings);
      bindings.delete(pendingId);
      return { pendingTopicBindings: bindings };
    }),
}));
