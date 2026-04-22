import type { ChangeEvent } from 'react';

import type {
  LocalDraftMediaItem,
  LocalPostDraft,
  PostView,
} from '@/lib/api';

import { PUBLIC_CHANNEL_REF, type DraftMediaItem, type DesktopShellState } from '@/shell/store';
import {
  canCreateRepostFromPost,
  messageFromError,
  publishedTopicIdForPost,
} from '@/shell/selectors';

import type {
  BoolStateDispatch,
  OpenThread,
  Setter,
  SyncRoute,
  Translate,
} from './shared';

type ComposeInteractionsParams = {
  activeTopic: string;
  buildImageDraftItem: (file: File) => Promise<DraftMediaItem>;
  buildVideoDraftItem: (file: File) => Promise<DraftMediaItem>;
  createOptimisticPost: (args: {
    createdAt: number;
    localId: string;
    draft: LocalPostDraft;
    draftMedia: LocalDraftMediaItem[];
    replyPost?: PostView | null;
    repostPost?: PostView | null;
  }) => PostView;
  insertOptimisticPost: (post: PostView) => void;
  openThread: OpenThread;
  releaseAllDirectMessageDraftPreviews: () => void;
  releaseAllDraftPreviews: () => void;
  releaseDirectMessageDraftPreview: (itemId: string) => void;
  releaseDraftPreview: (itemId: string) => void;
  rememberDirectMessageDraftPreview: (item: DraftMediaItem) => void;
  rememberDraftPreview: (item: DraftMediaItem) => void;
  restoreLocalDraft: (post: PostView) => void;
  shellChromeState: DesktopShellState['shellChromeState'];
  submitOptimisticPost: (post: PostView) => Promise<void>;
  syncRoute: SyncRoute;
  translate: Translate;
  setAttachmentInputKey: Setter<'attachmentInputKey'>;
  setAuthorError: Setter<'authorError'>;
  setComposer: Setter<'composer'>;
  setComposerError: Setter<'composerError'>;
  setDirectMessageAttachmentInputKey: Setter<'directMessageAttachmentInputKey'>;
  setDirectMessageDraftMediaItems: Setter<'directMessageDraftMediaItems'>;
  setDirectMessageError: Setter<'directMessageError'>;
  setDraftMediaItems: Setter<'draftMediaItems'>;
  setReplyTarget: Setter<'replyTarget'>;
  setRepostTarget: Setter<'repostTarget'>;
  setSelectedAuthor: Setter<'selectedAuthor'>;
  setSelectedAuthorPubkey: Setter<'selectedAuthorPubkey'>;
  setSelectedThread: Setter<'selectedThread'>;
  setShellChromeState: Setter<'shellChromeState'>;
  setThread: Setter<'thread'>;
  setComposeDialogOpen: BoolStateDispatch;
  setGameCreateDialogOpen: BoolStateDispatch;
  setLiveCreateDialogOpen: BoolStateDispatch;
};

export function createComposeInteractionsActions({
  activeTopic,
  buildImageDraftItem,
  buildVideoDraftItem,
  createOptimisticPost,
  insertOptimisticPost,
  openThread,
  releaseAllDirectMessageDraftPreviews,
  releaseAllDraftPreviews,
  releaseDirectMessageDraftPreview,
  releaseDraftPreview,
  rememberDirectMessageDraftPreview,
  rememberDraftPreview,
  restoreLocalDraft,
  shellChromeState,
  submitOptimisticPost,
  syncRoute,
  translate,
  setAttachmentInputKey,
  setAuthorError,
  setComposer,
  setComposerError,
  setDirectMessageAttachmentInputKey,
  setDirectMessageDraftMediaItems,
  setDirectMessageError,
  setDraftMediaItems,
  setReplyTarget,
  setRepostTarget,
  setSelectedAuthor,
  setSelectedAuthorPubkey,
  setSelectedThread,
  setShellChromeState,
  setThread,
  setComposeDialogOpen,
  setGameCreateDialogOpen,
  setLiveCreateDialogOpen,
}: ComposeInteractionsParams) {
  async function handleAttachmentSelection(event: ChangeEvent<HTMLInputElement>) {
    const files = Array.from(event.target.files ?? []);
    if (files.length === 0) {
      return;
    }

    const nextItems: DraftMediaItem[] = [];
    const failures: string[] = [];

    for (const file of files) {
      try {
        if (file.type.startsWith('image/')) {
          nextItems.push(await buildImageDraftItem(file));
          continue;
        }
        if (file.type.startsWith('video/')) {
          nextItems.push(await buildVideoDraftItem(file));
          continue;
        }
        failures.push(translate('common:errors.unsupportedAttachmentType', { name: file.name }));
      } catch (attachmentError) {
        failures.push(
          attachmentError instanceof Error
            ? attachmentError.message
            : translate('common:errors.failedToGenerateVideoPoster')
        );
      }
    }

    if (nextItems.length > 0) {
      nextItems.forEach(rememberDraftPreview);
      setDraftMediaItems((current) => [...current, ...nextItems]);
    }

    setComposerError(failures.length > 0 ? failures[0] : null);
    setAttachmentInputKey((value) => value + 1);
  }

  function handleRemoveDraftAttachment(itemId: string) {
    releaseDraftPreview(itemId);
    setDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

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
    } catch (attachmentError) {
      setDirectMessageError(
        messageFromError(attachmentError, translate('common:errors.failedToGenerateVideoPoster'))
      );
    } finally {
      setDirectMessageAttachmentInputKey((value) => value + 1);
    }
  }

  function handleRemoveDirectMessageDraftAttachment(itemId: string) {
    releaseDirectMessageDraftPreview(itemId);
    setDirectMessageDraftMediaItems((current) => current.filter((item) => item.id !== itemId));
  }

  function beginReply(post: PostView) {
    const threadId = post.root_id ?? post.object_id;
    setRepostTarget(null);
    setReplyTarget(post);
    setComposeDialogOpen(true);
    setSelectedThread(threadId);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('push', {
      selectedThread: threadId,
      selectedAuthorPubkey: null,
    });
    void openThread(threadId, { historyMode: 'replace' });
  }

  function clearReply() {
    setReplyTarget(null);
    setRepostTarget(null);
  }

  function clearRepost() {
    setRepostTarget(null);
  }

  function openNewPostDialog() {
    clearReply();
    clearRepost();
    setComposeDialogOpen(true);
  }

  function openFloatingActionDialog() {
    if (shellChromeState.activePrimarySection === 'live') {
      setLiveCreateDialogOpen(true);
      return;
    }
    if (shellChromeState.activePrimarySection === 'game') {
      setGameCreateDialogOpen(true);
      return;
    }
    openNewPostDialog();
  }

  async function handleSimpleRepost(post: PostView) {
    const sourceTopic = publishedTopicIdForPost(post);
    if (!sourceTopic || !canCreateRepostFromPost(post)) {
      setComposerError(translate('common:errors.failedToPublish'));
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
    setComposerError(null);
    setReplyTarget(null);
    setRepostTarget(null);
    setSelectedThread(null);
    setThread([]);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
    }));
    syncRoute('replace', {
      primarySection: 'timeline',
      selectedThread: null,
    });
    void submitOptimisticPost(optimisticPost);
  }

  function handleRestoreLocalPost(post: PostView) {
    restoreLocalDraft(post);
  }

  function handleRetryLocalPost(post: PostView) {
    if (post.local_state !== 'failed') {
      return;
    }
    setComposerError(null);
    void submitOptimisticPost(post);
  }

  function beginQuoteRepost(post: PostView) {
    if (!canCreateRepostFromPost(post)) {
      return;
    }
    releaseAllDraftPreviews();
    setDraftMediaItems([]);
    setAttachmentInputKey((value) => value + 1);
    setComposer('');
    setComposerError(null);
    setReplyTarget(null);
    setRepostTarget(post);
    setComposeDialogOpen(true);
    setSelectedAuthorPubkey(null);
    setSelectedAuthor(null);
    setAuthorError(null);
    syncRoute('replace', {
      selectedAuthorPubkey: null,
    });
  }

  return {
    handleAttachmentSelection,
    handleRemoveDraftAttachment,
    handleDirectMessageAttachmentSelection,
    handleRemoveDirectMessageDraftAttachment,
    beginReply,
    clearReply,
    clearRepost,
    openNewPostDialog,
    openFloatingActionDialog,
    handleSimpleRepost,
    handleRestoreLocalPost,
    handleRetryLocalPost,
    beginQuoteRepost,
  };
}
