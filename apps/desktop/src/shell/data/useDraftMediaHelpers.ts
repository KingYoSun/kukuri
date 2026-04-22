import { useCallback, type MutableRefObject } from 'react';

import {
  buildImageDraftItem,
  buildVideoDraftItem,
} from '@/shell/media';
import type { DraftMediaItem } from '@/shell/store';

type UseDraftMediaHelpersArgs = {
  draftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  directMessageDraftPreviewUrlRef: MutableRefObject<Map<string, string>>;
  draftSequenceRef: MutableRefObject<number>;
};

export function useDraftMediaHelpers({
  draftPreviewUrlRef,
  directMessageDraftPreviewUrlRef,
  draftSequenceRef,
}: UseDraftMediaHelpersArgs) {
  const nextDraftId = useCallback((): string => {
    draftSequenceRef.current += 1;
    return `draft-${draftSequenceRef.current}`;
  }, [draftSequenceRef]);

  const rememberDraftPreview = useCallback(
    (item: DraftMediaItem) => {
      draftPreviewUrlRef.current.set(item.id, item.preview_url);
    },
    [draftPreviewUrlRef]
  );

  const releaseDraftPreview = useCallback(
    (itemId: string) => {
      const previewUrl = draftPreviewUrlRef.current.get(itemId);
      if (!previewUrl) {
        return;
      }
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    },
    [draftPreviewUrlRef]
  );

  const releaseAllDraftPreviews = useCallback(() => {
    for (const [itemId, previewUrl] of draftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      draftPreviewUrlRef.current.delete(itemId);
    }
  }, [draftPreviewUrlRef]);

  const rememberDirectMessageDraftPreview = useCallback(
    (item: DraftMediaItem) => {
      directMessageDraftPreviewUrlRef.current.set(item.id, item.preview_url);
    },
    [directMessageDraftPreviewUrlRef]
  );

  const releaseDirectMessageDraftPreview = useCallback(
    (itemId: string) => {
      const previewUrl = directMessageDraftPreviewUrlRef.current.get(itemId);
      if (!previewUrl) {
        return;
      }
      URL.revokeObjectURL(previewUrl);
      directMessageDraftPreviewUrlRef.current.delete(itemId);
    },
    [directMessageDraftPreviewUrlRef]
  );

  const releaseAllDirectMessageDraftPreviews = useCallback(() => {
    for (const [itemId, previewUrl] of directMessageDraftPreviewUrlRef.current.entries()) {
      URL.revokeObjectURL(previewUrl);
      directMessageDraftPreviewUrlRef.current.delete(itemId);
    }
  }, [directMessageDraftPreviewUrlRef]);

  const buildImageItem = useCallback(
    async (file: File) => {
      return await buildImageDraftItem(file, nextDraftId);
    },
    [nextDraftId]
  );

  const buildVideoItem = useCallback(
    async (file: File) => {
      return await buildVideoDraftItem(file, nextDraftId);
    },
    [nextDraftId]
  );

  return {
    rememberDraftPreview,
    releaseDraftPreview,
    releaseAllDraftPreviews,
    rememberDirectMessageDraftPreview,
    releaseDirectMessageDraftPreview,
    releaseAllDirectMessageDraftPreviews,
    buildImageDraftItem: buildImageItem,
    buildVideoDraftItem: buildVideoItem,
  };
}
