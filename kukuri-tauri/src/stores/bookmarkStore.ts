import { create } from 'zustand';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';

interface BookmarkState {
  bookmarkedPostIds: Set<string>;
  isLoading: boolean;
  
  // Actions
  fetchBookmarks: () => Promise<void>;
  toggleBookmark: (postId: string) => Promise<void>;
  isBookmarked: (postId: string) => boolean;
}

export const useBookmarkStore = create<BookmarkState>((set, get) => ({
  bookmarkedPostIds: new Set(),
  isLoading: false,

  fetchBookmarks: async () => {
    set({ isLoading: true });
    try {
      const postIds = await TauriApi.getBookmarkedPostIds();
      set({ bookmarkedPostIds: new Set(postIds) });
    } catch (error) {
      errorHandler.log('Failed to fetch bookmarks', error, {
        context: 'BookmarkStore.fetchBookmarks',
        showToast: true,
        toastTitle: 'ブックマークの取得に失敗しました',
      });
    } finally {
      set({ isLoading: false });
    }
  },

  toggleBookmark: async (postId: string) => {
    const isCurrentlyBookmarked = get().isBookmarked(postId);
    
    try {
      if (isCurrentlyBookmarked) {
        await TauriApi.unbookmarkPost(postId);
        set((state) => {
          const newIds = new Set(state.bookmarkedPostIds);
          newIds.delete(postId);
          return { bookmarkedPostIds: newIds };
        });
      } else {
        await TauriApi.bookmarkPost(postId);
        set((state) => {
          const newIds = new Set(state.bookmarkedPostIds);
          newIds.add(postId);
          return { bookmarkedPostIds: newIds };
        });
      }
    } catch (error) {
      errorHandler.log('Failed to toggle bookmark', error, {
        context: 'BookmarkStore.toggleBookmark',
        showToast: true,
        toastTitle: isCurrentlyBookmarked ? 'ブックマークの解除に失敗しました' : 'ブックマークの追加に失敗しました',
      });
      throw error;
    }
  },

  isBookmarked: (postId: string) => {
    return get().bookmarkedPostIds.has(postId);
  },
}));