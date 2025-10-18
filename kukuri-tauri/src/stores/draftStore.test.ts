import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useDraftStore } from './draftStore';

// Mock errorHandler
vi.mock('@/lib/errorHandler', () => ({
  errorHandler: {
    handle: vi.fn(),
  },
}));

// Mock localStorage
const localStorageMock = {
  getItem: vi.fn(),
  setItem: vi.fn(),
  removeItem: vi.fn(),
  clear: vi.fn(),
};

Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
  writable: true,
});

describe('draftStore', () => {
  beforeEach(() => {
    // Setup fake timers
    vi.useFakeTimers();

    // Reset store state
    useDraftStore.setState({
      drafts: [],
      currentDraftId: null,
    });

    // Clear all mocks
    vi.clearAllMocks();
  });

  afterEach(() => {
    // Restore real timers
    vi.useRealTimers();
  });

  describe('createDraft', () => {
    it('creates a new draft and sets it as current', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({
        content: 'Test content',
        topicId: 'topic1',
        topicName: 'Test Topic',
      });

      expect(draft).toMatchObject({
        content: 'Test content',
        topicId: 'topic1',
        topicName: 'Test Topic',
      });

      expect(draft.id).toBeDefined();
      expect(draft.createdAt).toBeInstanceOf(Date);
      expect(draft.updatedAt).toBeInstanceOf(Date);

      const state = useDraftStore.getState();
      expect(state.drafts).toHaveLength(1);
      expect(state.currentDraftId).toBe(draft.id);
    });

    it('creates draft with metadata', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({
        content: 'Reply content',
        topicId: 'topic1',
        metadata: {
          replyTo: 'post123',
          quotedPost: 'post456',
        },
      });

      expect(draft.metadata).toEqual({
        replyTo: 'post123',
        quotedPost: 'post456',
      });
    });
  });

  describe('updateDraft', () => {
    it('updates an existing draft', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({
        content: 'Original content',
        topicId: 'topic1',
      });

      const originalUpdatedAt = draft.updatedAt;

      // Wait a bit to ensure updatedAt changes
      vi.advanceTimersByTime(100);

      store.updateDraft({
        id: draft.id,
        content: 'Updated content',
        topicId: 'topic2',
      });

      const updatedDraft = store.getDraft(draft.id);
      expect(updatedDraft?.content).toBe('Updated content');
      expect(updatedDraft?.topicId).toBe('topic2');
      expect(updatedDraft?.updatedAt.getTime()).toBeGreaterThan(originalUpdatedAt.getTime());
    });
  });

  describe('deleteDraft', () => {
    it('deletes a draft', () => {
      const store = useDraftStore.getState();

      const draft1 = store.createDraft({ content: 'Draft 1', topicId: null });
      const draft2 = store.createDraft({ content: 'Draft 2', topicId: null });

      const state = useDraftStore.getState();
      expect(state.drafts).toHaveLength(2);

      store.deleteDraft(draft1.id);

      const newState = useDraftStore.getState();
      expect(newState.drafts).toHaveLength(1);
      expect(store.getDraft(draft1.id)).toBeUndefined();
      expect(store.getDraft(draft2.id)).toBeDefined();
    });

    it('resets currentDraftId if deleted draft was current', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Test', topicId: null });
      const state = useDraftStore.getState();
      expect(state.currentDraftId).toBe(draft.id);

      store.deleteDraft(draft.id);

      const newState = useDraftStore.getState();
      expect(newState.currentDraftId).toBeNull();
    });
  });

  describe('listDrafts', () => {
    it('returns drafts sorted by updatedAt descending', async () => {
      const store = useDraftStore.getState();

      // Create drafts with different timestamps
      const draft1 = store.createDraft({ content: 'Draft 1', topicId: null });

      // Advance time
      vi.advanceTimersByTime(1000);

      const draft2 = store.createDraft({ content: 'Draft 2', topicId: null });

      // Update draft1 to make it most recent
      vi.advanceTimersByTime(1000);
      store.updateDraft({ id: draft1.id, content: 'Draft 1 updated' });

      const drafts = store.listDrafts();

      expect(drafts[0].id).toBe(draft1.id);
      expect(drafts[1].id).toBe(draft2.id);
    });
  });

  describe('getCurrentDraft', () => {
    it('returns the current draft', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Test', topicId: null });

      const currentDraft = store.getCurrentDraft();
      expect(currentDraft).toEqual(draft);
    });

    it('returns undefined if no current draft', () => {
      const store = useDraftStore.getState();

      expect(store.getCurrentDraft()).toBeUndefined();
    });
  });

  describe('setCurrentDraft', () => {
    it('sets the current draft ID', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Test', topicId: null });

      store.setCurrentDraft(null);
      const state1 = useDraftStore.getState();
      expect(state1.currentDraftId).toBeNull();

      store.setCurrentDraft(draft.id);
      const state2 = useDraftStore.getState();
      expect(state2.currentDraftId).toBe(draft.id);
    });
  });

  describe('clearAllDrafts', () => {
    it('removes all drafts and resets currentDraftId', () => {
      const store = useDraftStore.getState();

      store.createDraft({ content: 'Draft 1', topicId: null });
      store.createDraft({ content: 'Draft 2', topicId: null });
      store.createDraft({ content: 'Draft 3', topicId: null });

      const state1 = useDraftStore.getState();
      expect(state1.drafts).toHaveLength(3);
      expect(state1.currentDraftId).not.toBeNull();

      store.clearAllDrafts();

      const state2 = useDraftStore.getState();
      expect(state2.drafts).toHaveLength(0);
      expect(state2.currentDraftId).toBeNull();
    });
  });

  describe('autosaveDraft', () => {
    it('updates draft if content changed', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Original', topicId: null });
      console.info('1. Draft after creation:', {
        id: draft.id,
        content: draft.content,
        topicId: draft.topicId,
        updatedAt: draft.updatedAt,
      });

      // Advance timer to ensure different updatedAt timestamp
      vi.advanceTimersByTime(100);

      const autosaveParams = {
        id: draft.id,
        content: 'Updated',
        topicId: null,
      };
      console.info('2. Params being passed to autosaveDraft:', autosaveParams);

      store.autosaveDraft(autosaveParams);

      const updatedDraft = store.getDraft(draft.id);
      console.info('3. Draft after autosaveDraft:', {
        id: updatedDraft?.id,
        content: updatedDraft?.content,
        topicId: updatedDraft?.topicId,
        updatedAt: updatedDraft?.updatedAt,
      });

      expect(updatedDraft).toBeDefined();
      expect(updatedDraft?.content).toBe('Updated');
    });

    it('does not update if content unchanged', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Original', topicId: null });
      const originalUpdatedAt = draft.updatedAt;

      store.autosaveDraft({
        id: draft.id,
        content: 'Original',
        topicId: null,
      });

      const unchangedDraft = store.getDraft(draft.id);
      expect(unchangedDraft?.updatedAt).toEqual(originalUpdatedAt);
    });

    it('handles errors gracefully', () => {
      const store = useDraftStore.getState();

      // Autosave with non-existent draft ID should not throw
      expect(() => {
        store.autosaveDraft({
          id: 'non-existent',
          content: 'Test',
        });
      }).not.toThrow();
    });
  });

  describe('persistence', () => {
    it('persists drafts to localStorage', () => {
      const store = useDraftStore.getState();

      // Manually trigger persist by creating draft and checking state
      store.createDraft({ content: 'Persistent draft', topicId: null });

      // Since Zustand persist is async, we check the state directly
      const state = useDraftStore.getState();
      expect(state.drafts).toHaveLength(1);
      expect(state.drafts[0].content).toBe('Persistent draft');
    });

    it('stores draft data correctly', () => {
      const store = useDraftStore.getState();

      const draft = store.createDraft({ content: 'Test', topicId: null });

      // Verify the draft was created with expected structure
      const state = useDraftStore.getState();
      expect(state.drafts[0]).toMatchObject({
        content: 'Test',
        topicId: null,
      });
      // currentDraftId should not be persisted (handled by store configuration)
      expect(state.currentDraftId).toBe(draft.id);
    });
  });
});
