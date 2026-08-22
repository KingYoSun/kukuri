import type {
  AttachmentView,
  DesktopApi,
  LocalDraftMediaItem,
  LocalPostDraft,
  PostView,
} from '@/lib/api';
import {
  activeTimelineStorageKey,
  PUBLIC_CHANNEL_REF,
  timelineStorageKeyForChannel,
  type DesktopShellState,
  type DesktopShellStoreApi,
  type DraftMediaItem,
} from '@/shell/store';
import { publishedTopicIdForPost } from '@/shell/presentation';
import { setRecordEntry, updateRecordEntry } from '@/shell/stateUpdates';

import type { BoolStateDispatch, Setter, SyncRoute, Translate } from './shared';

type OptimisticPostActionsParams = {
  api: DesktopApi;
  activeTopic: string;
  localAuthorPubkey: string;
  localProfile: DesktopShellState['localProfile'];
  refreshVisibleTimelineAfterPublish: (
    topic: string,
    currentThread: string | null,
    scopeChannelId?: string | null
  ) => Promise<void>;
  releaseDraftPreview: (itemId: string) => void;
  rememberDraftPreview: (item: DraftMediaItem) => void;
  storeApi: DesktopShellStoreApi;
  syncRoute: SyncRoute;
  translate: Translate;
  setActiveTopic: Setter<'activeTopic'>;
  setAttachmentInputKey: Setter<'attachmentInputKey'>;
  setComposeChannelByTopic: Setter<'composeChannelByTopic'>;
  setComposeDialogOpen: BoolStateDispatch;
  setComposer: Setter<'composer'>;
  setComposerError: Setter<'composerError'>;
  setDraftMediaItems: Setter<'draftMediaItems'>;
  setProfileTimeline: Setter<'profileTimeline'>;
  setPublicTimelinesByTopic: Setter<'publicTimelinesByTopic'>;
  setReplyTarget: Setter<'replyTarget'>;
  setRepostTarget: Setter<'repostTarget'>;
  setSelectedAuthorTimeline: Setter<'selectedAuthorTimeline'>;
  setSelectedChannelIdByTopic: Setter<'selectedChannelIdByTopic'>;
  setSelectedThread: Setter<'selectedThread'>;
  setShellChromeState: Setter<'shellChromeState'>;
  setThread: Setter<'thread'>;
  setTimelinesByKey: Setter<'timelinesByKey'>;
};

export function createOptimisticPostActions({
  api,
  activeTopic,
  localAuthorPubkey,
  localProfile,
  refreshVisibleTimelineAfterPublish,
  releaseDraftPreview,
  rememberDraftPreview,
  storeApi,
  syncRoute,
  translate,
  setActiveTopic,
  setAttachmentInputKey,
  setComposeChannelByTopic,
  setComposeDialogOpen,
  setComposer,
  setComposerError,
  setDraftMediaItems,
  setProfileTimeline,
  setPublicTimelinesByTopic,
  setReplyTarget,
  setRepostTarget,
  setSelectedAuthorTimeline,
  setSelectedChannelIdByTopic,
  setSelectedThread,
  setShellChromeState,
  setThread,
  setTimelinesByKey,
}: OptimisticPostActionsParams) {
  function cloneDraftMediaItems(items: DraftMediaItem[]): LocalDraftMediaItem[] {
    return items.map((item) => ({
      id: item.id,
      source_name: item.source_name,
      preview_url: item.preview_url,
      attachments: item.attachments.map((attachment) => ({ ...attachment })),
    }));
  }

  function attachmentViewsFromDraftMediaItems(
    localId: string,
    items: LocalDraftMediaItem[]
  ): AttachmentView[] {
    let attachmentIndex = 0;
    return items.flatMap((item) =>
      item.attachments.map((attachment) => ({
        hash: `${localId}-attachment-${attachmentIndex++}`,
        mime: attachment.mime,
        bytes: attachment.byte_size,
        role: attachment.role ?? 'image_original',
        status: 'Available',
      }))
    );
  }

  function prependPost(posts: PostView[], post: PostView) {
    return [post, ...posts.filter((current) => current.object_id !== post.object_id)];
  }

  function patchLocalPosts(
    localId: string,
    updater: (post: PostView) => PostView,
    topicId: string = activeTopic
  ) {
    const patch = (posts: PostView[]) =>
      posts.map((post) => (post.local_id === localId ? updater(post) : post));

    setTimelinesByKey((current) => {
      let changed = false;
      const next = { ...current };
      for (const [key, posts] of Object.entries(current)) {
        if (!key.startsWith(`${topicId}::`) || !posts.some((post) => post.local_id === localId)) {
          continue;
        }
        next[key] = patch(posts);
        changed = true;
      }
      return changed ? next : current;
    });
    setPublicTimelinesByTopic(updateRecordEntry(topicId, (prev) => patch(prev ?? [])));
    setThread((current) => patch(current));
    setProfileTimeline((current) => patch(current));
    setSelectedAuthorTimeline((current) => patch(current));
  }

  function findKnownPost(objectId: string): PostView | null {
    const currentState = storeApi.getState();
    const lists = [
      currentState.thread,
      currentState.timelinesByKey[
        activeTimelineStorageKey(currentState, currentState.activeTopic)
      ] ?? [],
      currentState.publicTimelinesByTopic[currentState.activeTopic] ?? [],
      currentState.profileTimeline,
      currentState.selectedAuthorTimeline,
    ];
    for (const posts of lists) {
      const match = posts.find((post) => post.object_id === objectId);
      if (match) {
        return match;
      }
    }
    return null;
  }

  function restoreLocalDraft(post: PostView) {
    const draft = post.local_draft;
    if (!draft) {
      return;
    }
    const draftMedia = cloneDraftMediaItems(post.local_draft_media_items ?? []);
    for (const item of draftMedia) {
      rememberDraftPreview(item as DraftMediaItem);
    }
    if (draft.topic !== activeTopic) {
      setActiveTopic(draft.topic);
    }
    setComposer(draft.content);
    setDraftMediaItems(draftMedia as DraftMediaItem[]);
    setAttachmentInputKey((value) => value + 1);
    setComposeChannelByTopic(
      setRecordEntry(
        draft.topic,
        draft.kind === 'repost' ? PUBLIC_CHANNEL_REF : (draft.channel_ref ?? PUBLIC_CHANNEL_REF)
      )
    );
    if (draft.kind === 'repost' && draft.source_object_id) {
      setRepostTarget(findKnownPost(draft.source_object_id));
      setReplyTarget(null);
    } else if (draft.reply_to) {
      setReplyTarget(findKnownPost(draft.reply_to));
      setRepostTarget(null);
      setSelectedThread(post.root_id ?? draft.reply_to);
    } else {
      setReplyTarget(null);
      setRepostTarget(null);
      const channelRef = draft.channel_ref;
      if (channelRef && channelRef.kind === 'private_channel') {
        setSelectedChannelIdByTopic(setRecordEntry(draft.topic, channelRef.channel_id));
      }
    }
    setComposerError(post.local_error ?? null);
    setComposeDialogOpen(true);
    setShellChromeState((current) => ({
      ...current,
      activePrimarySection: 'timeline',
    }));
    syncRoute('replace', {
      activeTopic: draft.topic,
      primarySection: 'timeline',
      selectedThread: draft.reply_to ? post.root_id ?? draft.reply_to : null,
    });
  }

  function createOptimisticPost(args: {
    createdAt: number;
    localId: string;
    draft: LocalPostDraft;
    draftMedia: LocalDraftMediaItem[];
    replyPost?: PostView | null;
    repostPost?: PostView | null;
  }): PostView {
    const { createdAt, localId, draft, draftMedia, replyPost = null, repostPost = null } = args;
    const isRepost = draft.kind === 'repost' && repostPost;
    const channelId =
      draft.kind === 'post' && draft.channel_ref?.kind === 'private_channel'
        ? draft.channel_ref.channel_id
        : null;
    const channelLabel = channelId
      ? storeApi
          .getState()
          .joinedChannelsByTopic[draft.topic]?.find(
            (channel) => channel.channel_id === channelId
          )?.label
      : null;
    const rootId = replyPost ? replyPost.root_id ?? replyPost.object_id : localId;
    return {
      object_id: localId,
      envelope_id: localId,
      author_pubkey: localAuthorPubkey,
      author_name: localProfile?.name ?? null,
      author_display_name: localProfile?.display_name ?? null,
      following: false,
      followed_by: false,
      mutual: false,
      friend_of_friend: false,
      object_kind: isRepost ? 'repost' : replyPost ? 'comment' : 'post',
      content: draft.content,
      content_status: 'Available',
      attachments: attachmentViewsFromDraftMediaItems(localId, draftMedia),
      created_at: createdAt,
      reply_to: replyPost?.object_id ?? null,
      reply_preview: replyPost
        ? {
            object_id: replyPost.object_id,
            topic: publishedTopicIdForPost(replyPost) ?? draft.topic,
            author: {
              pubkey: replyPost.author_pubkey,
              name: replyPost.author_name ?? null,
              display_name: replyPost.author_display_name ?? null,
              picture: replyPost.author_picture ?? null,
              picture_asset: replyPost.author_picture_asset ?? null,
            },
            content: replyPost.content,
            attachments: replyPost.attachments.map((attachment) => ({ ...attachment })),
            root_id: replyPost.root_id ?? null,
            reply_to: replyPost.reply_to ?? null,
          }
        : null,
      root_id: isRepost ? null : rootId,
      published_topic_id: draft.topic,
      origin_topic_id: draft.topic,
      repost_of: repostPost
        ? {
            source_object_id: repostPost.object_id,
            source_topic_id: publishedTopicIdForPost(repostPost) ?? draft.topic,
            source_author_pubkey: repostPost.author_pubkey,
            source_author_name: repostPost.author_name ?? null,
            source_author_display_name: repostPost.author_display_name ?? null,
            source_object_kind: repostPost.object_kind,
            content: repostPost.content,
            attachments: repostPost.attachments.map((attachment) => ({ ...attachment })),
            reply_to: repostPost.reply_to ?? null,
            root_id: repostPost.root_id ?? null,
          }
        : null,
      repost_commentary: repostPost ? (draft.content.trim() || null) : null,
      is_threadable: repostPost ? Boolean(draft.content.trim()) : true,
      channel_id: channelId,
      audience_label:
        replyPost?.audience_label ??
        (channelId ? channelLabel ?? 'Private channel' : 'Public'),
      reaction_summary: [],
      my_reactions: [],
      local_id: localId,
      local_state: 'pending',
      local_error: null,
      server_object_id: null,
      local_draft: {
        ...draft,
        attachments: draft.attachments?.map((attachment) => ({ ...attachment })) ?? [],
      },
      local_draft_media_items: draftMedia,
    };
  }

  function insertOptimisticPost(post: PostView) {
    const currentState = storeApi.getState();
    const topicId = post.published_topic_id ?? activeTopic;
    const timelineKey = timelineStorageKeyForChannel(topicId, post.channel_id ?? null);
    setTimelinesByKey(updateRecordEntry(timelineKey, (prev) => prependPost(prev ?? [], post)));
    if (!post.channel_id) {
      setPublicTimelinesByTopic(updateRecordEntry(topicId, (prev) => prependPost(prev ?? [], post)));
    }
    if (post.root_id && currentState.selectedThread === post.root_id) {
      setThread((current) => prependPost(current, post));
    }
    if (
      !post.channel_id &&
      localProfile &&
      currentState.shellChromeState.activePrimarySection === 'profile'
    ) {
      setProfileTimeline((current) => prependPost(current, post));
    }
    if (
      !post.channel_id &&
      currentState.selectedAuthorPubkey === localAuthorPubkey &&
      currentState.shellChromeState.activePrimarySection === 'timeline'
    ) {
      setSelectedAuthorTimeline((current) => prependPost(current, post));
    }
  }

  async function submitOptimisticPost(post: PostView): Promise<boolean> {
    const draft = post.local_draft;
    const draftMedia = cloneDraftMediaItems(post.local_draft_media_items ?? []);
    if (!draft || !post.local_id) {
      return false;
    }
    patchLocalPosts(
      post.local_id,
      (current) => ({
        ...current,
        local_state: 'pending',
        local_error: null,
      }),
      draft.topic
    );
    try {
      const serverObjectId =
        draft.kind === 'repost' && draft.source_topic && draft.source_object_id
          ? await api.createRepost(
              draft.topic,
              draft.source_topic,
              draft.source_object_id,
              draft.content.trim() || null
            )
          : await api.createPost(
              draft.topic,
              draft.content,
              draft.reply_to ?? null,
              draft.attachments ?? [],
              draft.channel_ref ?? PUBLIC_CHANNEL_REF
            );
      for (const item of draftMedia) {
        releaseDraftPreview(item.id);
      }
      patchLocalPosts(
        post.local_id,
        (current) => ({
          ...current,
          local_state: 'syncing',
          local_error: null,
          server_object_id: serverObjectId,
        }),
        draft.topic
      );
      void refreshVisibleTimelineAfterPublish(
        draft.topic,
        draft.reply_to ? post.root_id ?? draft.reply_to : null,
        draft.channel_ref?.kind === 'private_channel' ? draft.channel_ref.channel_id : null
      );
      return true;
    } catch (publishError) {
      const message =
        publishError instanceof Error
          ? publishError.message
          : translate('common:errors.failedToPublish');
      patchLocalPosts(
        post.local_id,
        (current) => ({
          ...current,
          local_state: 'failed',
          local_error: message,
        }),
        draft.topic
      );
      setComposerError(message);
      return false;
    }
  }

  return {
    cloneDraftMediaItems,
    createOptimisticPost,
    insertOptimisticPost,
    restoreLocalDraft,
    submitOptimisticPost,
  };
}
