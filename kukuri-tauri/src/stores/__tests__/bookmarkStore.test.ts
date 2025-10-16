import { vi, describe, it, expect, beforeEach } from 'vitest';
import { useBookmarkStore } from '@/stores/bookmarkStore';
import { TauriApi } from '@/lib/api/tauri';
import { errorHandler } from '@/lib/errorHandler';

vi.mock('@/lib/api/tauri');
vi.mock('@/lib/errorHandler');

const mockTauriApi = TauriApi as unknown as {
  getBookmarkedPostIds: ReturnType<typeof vi.fn>;
  bookmarkPost: ReturnType<typeof vi.fn>;
  unbookmarkPost: ReturnType<typeof vi.fn>;
};

const mockErrorHandler = errorHandler as unknown as {
  log: ReturnType<typeof vi.fn>;
};

describe('bookmarkStore', () => {
  beforeEach(() => {
    useBookmarkStore.setState({
      bookmarkedPostIds: new Set(),
      isLoading: false,
    });
    vi.clearAllMocks();
  });

  describe('fetchBookmarks', () => {
    it('should fetch and set bookmarked post IDs', async () => {
      const mockPostIds = ['post1', 'post2', 'post3'];
      mockTauriApi.getBookmarkedPostIds = vi.fn().mockResolvedValue(mockPostIds);

      await useBookmarkStore.getState().fetchBookmarks();

      expect(mockTauriApi.getBookmarkedPostIds).toHaveBeenCalled();
      expect(useBookmarkStore.getState().bookmarkedPostIds).toEqual(new Set(mockPostIds));
      expect(useBookmarkStore.getState().isLoading).toBe(false);
    });

    it('should handle fetch errors', async () => {
      const error = new Error('Failed to fetch');
      mockTauriApi.getBookmarkedPostIds = vi.fn().mockRejectedValue(error);
      mockErrorHandler.log = vi.fn();

      await useBookmarkStore.getState().fetchBookmarks();

      expect(mockErrorHandler.log).toHaveBeenCalledWith(
        'Failed to fetch bookmarks',
        error,
        expect.objectContaining({
          context: 'BookmarkStore.fetchBookmarks',
          showToast: true,
          toastTitle: 'ブックマークの取得に失敗しました',
        }),
      );
      expect(useBookmarkStore.getState().isLoading).toBe(false);
    });
  });

  describe('toggleBookmark', () => {
    it('should add bookmark when post is not bookmarked', async () => {
      mockTauriApi.bookmarkPost = vi.fn().mockResolvedValue(undefined);
      const postId = 'post1';

      await useBookmarkStore.getState().toggleBookmark(postId);

      expect(mockTauriApi.bookmarkPost).toHaveBeenCalledWith(postId);
      expect(useBookmarkStore.getState().bookmarkedPostIds.has(postId)).toBe(true);
    });

    it('should remove bookmark when post is bookmarked', async () => {
      mockTauriApi.unbookmarkPost = vi.fn().mockResolvedValue(undefined);
      const postId = 'post1';
      useBookmarkStore.setState({
        bookmarkedPostIds: new Set([postId]),
      });

      await useBookmarkStore.getState().toggleBookmark(postId);

      expect(mockTauriApi.unbookmarkPost).toHaveBeenCalledWith(postId);
      expect(useBookmarkStore.getState().bookmarkedPostIds.has(postId)).toBe(false);
    });

    it('should handle toggle errors when adding bookmark', async () => {
      const error = new Error('Failed to add');
      mockTauriApi.bookmarkPost = vi.fn().mockRejectedValue(error);
      mockErrorHandler.log = vi.fn();
      const postId = 'post1';

      await expect(useBookmarkStore.getState().toggleBookmark(postId)).rejects.toThrow(error);

      expect(mockErrorHandler.log).toHaveBeenCalledWith(
        'Failed to toggle bookmark',
        error,
        expect.objectContaining({
          context: 'BookmarkStore.toggleBookmark',
          showToast: true,
          toastTitle: 'ブックマークの追加に失敗しました',
        }),
      );
    });

    it('should handle toggle errors when removing bookmark', async () => {
      const error = new Error('Failed to remove');
      mockTauriApi.unbookmarkPost = vi.fn().mockRejectedValue(error);
      mockErrorHandler.log = vi.fn();
      const postId = 'post1';
      useBookmarkStore.setState({
        bookmarkedPostIds: new Set([postId]),
      });

      await expect(useBookmarkStore.getState().toggleBookmark(postId)).rejects.toThrow(error);

      expect(mockErrorHandler.log).toHaveBeenCalledWith(
        'Failed to toggle bookmark',
        error,
        expect.objectContaining({
          context: 'BookmarkStore.toggleBookmark',
          showToast: true,
          toastTitle: 'ブックマークの解除に失敗しました',
        }),
      );
    });
  });

  describe('isBookmarked', () => {
    it('should return true when post is bookmarked', () => {
      const postId = 'post1';
      useBookmarkStore.setState({
        bookmarkedPostIds: new Set([postId]),
      });

      expect(useBookmarkStore.getState().isBookmarked(postId)).toBe(true);
    });

    it('should return false when post is not bookmarked', () => {
      const postId = 'post1';

      expect(useBookmarkStore.getState().isBookmarked(postId)).toBe(false);
    });
  });
});
