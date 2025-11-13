import { describe, it, expect, beforeEach, vi } from 'vitest';
import { fireEvent, screen, waitFor } from '@testing-library/react';
import React from 'react';
import type { Post } from '@/stores';
import {
  mockPost,
  renderWithQueryClient,
  deletePostMutationMock,
  toastMock,
  offlineStoreState,
  useDeletePostMock,
} from './__utils__/postCardTestUtils';
import { PostCard } from '@/components/posts/PostCard';

describe('PostCard delete flow (offline)', () => {
  const createOwnPost = (): Post => ({
    ...mockPost,
    author: {
      ...mockPost.author,
      pubkey: 'user-pubkey',
      npub: 'npub1user',
    },
  });

  beforeEach(() => {
    deletePostMutationMock.mutate.mockReset();
    deletePostMutationMock.manualRetryDelete.mockReset();
    deletePostMutationMock.isPending = false;
    toastMock.success.mockReset();
    toastMock.error.mockReset();
    offlineStoreState.isOnline = false;
    useDeletePostMock.mockReset();
    useDeletePostMock.mockReturnValue(deletePostMutationMock);
  });

  it('オフライン時は削除予約のトーストを表示する', async () => {
    const ownPost = createOwnPost();
    useDeletePostMock.mockReturnValue({
      ...deletePostMutationMock,
      mutate: (_post: Post, options) => {
        options?.onSuccess?.(_post);
        options?.onSettled?.();
        toastMock.success('削除は接続復旧後に反映されます');
      },
    });

    renderWithQueryClient(<PostCard post={ownPost} />);

    fireEvent.click(screen.getByRole('button', { name: /削除/ }));
    fireEvent.click(screen.getByText('削除する'));

    await waitFor(() => {
      expect(toastMock.success).toHaveBeenCalledWith('削除は接続復旧後に反映されます');
    });
  });

  it('再送中は削除アクションを重複実行しない', async () => {
    const ownPost = createOwnPost();
    const mutateSpy = vi.fn((_post: Post, options?: { onSettled?: () => void }) => {
      options?.onSettled?.();
    });
    useDeletePostMock.mockReturnValue({
      ...deletePostMutationMock,
      mutate: mutateSpy,
      isPending: true,
    });

    renderWithQueryClient(<PostCard post={ownPost} />);
    const deleteButton = screen.getByRole('button', { name: /削除/ });
    fireEvent.click(deleteButton);
    fireEvent.click(screen.getByText('削除する'));

    await waitFor(() => {
      expect(mutateSpy).not.toHaveBeenCalled();
    });
  });
});
