import { create } from 'zustand';
import { persist } from 'zustand/middleware';

import type { PostDraft, CreateDraftParams, UpdateDraftParams } from '@/types/draft';
import { errorHandler } from '@/lib/errorHandler';

interface DraftStore {
  drafts: PostDraft[];
  currentDraftId: string | null;

  // Actions
  createDraft: (params: CreateDraftParams) => PostDraft;
  updateDraft: (params: UpdateDraftParams) => void;
  deleteDraft: (id: string) => void;
  getDraft: (id: string) => PostDraft | undefined;
  setCurrentDraft: (id: string | null) => void;
  getCurrentDraft: () => PostDraft | undefined;
  listDrafts: () => PostDraft[];
  clearAllDrafts: () => void;

  // Autosave
  autosaveDraft: (params: UpdateDraftParams) => void;
}

const generateId = () => {
  return `draft_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
};

export const useDraftStore = create<DraftStore>()(
  persist(
    (set, get) => ({
      drafts: [],
      currentDraftId: null,

      createDraft: (params) => {
        const draft: PostDraft = {
          id: generateId(),
          content: params.content,
          topicId: params.topicId,
          topicName: params.topicName,
          createdAt: new Date(),
          updatedAt: new Date(),
          metadata: params.metadata,
        };

        set((state) => ({
          drafts: [draft, ...state.drafts],
          currentDraftId: draft.id,
        }));

        return draft;
      },

      updateDraft: (params) => {
        set((state) => ({
          drafts: state.drafts.map((draft) =>
            draft.id === params.id
              ? {
                  ...draft,
                  ...params,
                  updatedAt: new Date(),
                }
              : draft,
          ),
        }));
      },

      deleteDraft: (id) => {
        set((state) => ({
          drafts: state.drafts.filter((draft) => draft.id !== id),
          currentDraftId: state.currentDraftId === id ? null : state.currentDraftId,
        }));
      },

      getDraft: (id) => {
        return get().drafts.find((draft) => draft.id === id);
      },

      setCurrentDraft: (id) => {
        set({ currentDraftId: id });
      },

      getCurrentDraft: () => {
        const { currentDraftId, drafts } = get();
        if (!currentDraftId) return undefined;
        return drafts.find((draft) => draft.id === currentDraftId);
      },

      listDrafts: () => {
        return get().drafts.sort(
          (a, b) => new Date(b.updatedAt).getTime() - new Date(a.updatedAt).getTime(),
        );
      },

      clearAllDrafts: () => {
        set({ drafts: [], currentDraftId: null });
      },

      autosaveDraft: (params) => {
        try {
          const state = get();
          // Direct access to drafts array
          const existingDraft = state.drafts.find((draft) => draft.id === params.id);

          if (existingDraft) {
            // Only update if content has changed
            const shouldUpdate =
              existingDraft.content !== params.content || existingDraft.topicId !== params.topicId;

            if (shouldUpdate) {
              // Update draft directly by manipulating state
              set((currentState) => ({
                drafts: currentState.drafts.map((draft) =>
                  draft.id === params.id
                    ? {
                        ...draft,
                        ...params,
                        updatedAt: new Date(),
                      }
                    : draft,
                ),
              }));
            }
          }
        } catch (error) {
          errorHandler.log('Autosave draft failed', error, {
            context: 'Autosave draft failed',
          });
        }
      },
    }),
    {
      name: 'kukuri-drafts',
      partialize: (state) => ({
        drafts: state.drafts,
        // Don't persist currentDraftId to avoid confusion on reload
      }),
    },
  ),
);
