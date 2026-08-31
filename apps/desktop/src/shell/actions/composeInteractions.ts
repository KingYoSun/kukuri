import type { ChangeEvent } from 'react';
import type { PostView } from '@/lib/api';

import { PUBLIC_CHANNEL_REF, type DraftMediaItem } from '@/shell/store';
import { canCreateRepostFromPost, publishedTopicIdForPost } from '@/shell/presentation';

import type {
  Setter,
  SyncRoute,
  Translate,
} from './shared';

type ComposeInteractionsParams = {
  activeTopic: string;
  buildImageDraftItem: (file: File) => Promise<DraftMediaItem>;
  buildVideoDraftItem: (file: File) => Promise<DraftMediaItem>;
  createOptimisticPost: ReturnType<
    typeof import('./optimisticPosts').createOptimisticPostActions
  >['createOptimisticPost'];
  insertOptimisticPost: (post: PostView) => void;
  releaseAllDirectMessageDraftPreviews: () => void;
  releaseDirectMessageDraftPreview: (itemId: string) => void;
  rememberDirectMessageDraftPreview: (item: DraftMediaItem) => void;
  submitOptimisticPost: (post: PostView) => Promise<void>;
  syncRoute: SyncRoute;
  translate: Translate;
  setDirectMessageAttachmentInputKey: Setter<'directMessageAttachmentInputKey'>;
  setDirectMessageDraftMediaItems: Setter<'directMessageDraftMediaItems'>;
  setDirectMessageError: Setter<'directMessageError'>;
  setError: Setter<'error'>;
  setSelectedThread: Setter<'selectedThread'>;
  setShellChromeState: Setter<'shellChromeState'>;
};

export function createComposeInteractionsActions({
  activeTopic,
  buildImageDraftItem,
  buildVideoDraftItem,
  createOptimisticPost,
  insertOptimisticPost,
  releaseAllDirectMessageDraftPreviews,
  releaseDirectMessageDraftPreview,
  rememberDirectMessageDraftPreview,
  submitOptimisticPost,
  syncRoute,
  translate,
  setDirectMessageAttachmentInputKey,
  setDirectMessageDraftMediaItems,
  setDirectMessageError,
  setError,
  setSelectedThread,
  setShellChromeState,
}: ComposeInteractionsParams) {
  async function handleDirectMessageAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (!file) {
      return;
    }

    try {
      const nextItem = file.type.startsWith('image/')
        ? await buildImageDraftItem(file)
        : file.type.startsWith('video/')
          ? await buildVideoDraftItem(file)
          : null;
      if (!nextItem) {
        setDirectMessageError(
          translate('common:errors.unsupportedAttachmentType', { name: file.name })
        );
      } else {
        releaseAllDirectMessageDraftPreviews();
        rememberDirectMessageDraftPreview(nextItem);
        setDirectMessageDraftMediaItems([nextItem]);
        setDirectMessageError(null);
      }
    } catch {
      setDirectMessageError(translate('common:errors.failedToGenerateVideoPoster'));
    } finally {
      setDirectMessageAttachmentInputKey((value) => value + 1);
    }
  }

  function handleRemoveDirectMessageDraftAttachment(itemId: string) {
    releaseDirectMessageDraftPreview(itemId);
    setDirectMessageDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  async function handleSimpleRepost(post: PostView) {
    const sourceTopic = publishedTopicIdForPost(post);
    if (!sourceTopic || !canCreateRepostFromPost(post)) {
      setError(translate('common:errors.failedToPublish'));
      return;
    }
    const createdAt = Math.floor(Date.now() / 1000);
    const localId = `local-post:${Date.now()}:${Math.random().toString(16).slice(2)}`;
    const optimisticPost = createOptimisticPost({
      createdAt,
      localId,
      draft: {
        kind: 'repost',
        topic: activeTopic,
        content: '',
        source_topic: sourceTopic,
        source_object_id: post.object_id,
        channel_ref: PUBLIC_CHANNEL_REF,
      },
      draftMedia: [],
      repostPost: post,
    });
    insertOptimisticPost(optimisticPost);
    setError(null);
    setSelectedThread(null);
    setShellChromeState((current) => ({
      ...current,
    }));
    syncRoute('replace', {
      primarySection: 'timeline',
      selectedThread: null,
    });
    void submitOptimisticPost(optimisticPost);
  }

  function handleRetryLocalPost(post: PostView) {
    if (post.local_state !== 'failed') {
      return;
    }
    setError(null);
    void submitOptimisticPost(post);
  }

  return {
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    handleSimpleRepost,
    handleRetryLocalPost,
  };
}
