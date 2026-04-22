import type { ChangeEvent, FormEvent } from 'react';

import { AuthorAvatar } from '@/components/core/AuthorAvatar';
import { AuthorDetailCard } from '@/components/core/AuthorDetailCard';
import { AuthorIdentityButton } from '@/components/core/AuthorIdentityButton';
import { ComposerDraftPreviewList } from '@/components/core/ComposerDraftPreviewList';
import { ThreadPanel } from '@/components/core/ThreadPanel';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

import type {
  NotificationView,
  PostView,
  ReactionKeyInput,
} from '@/lib/api';
import { formatLocalizedTime } from '@/i18n/format';
import type { SupportedLocale } from '@/i18n';
import { type InternalSmartReference } from '@/lib/internalLinks';
import { ContextPane } from '@/components/shell/ContextPane';
import { SHELL_CONTEXT_ID, useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import {
  authorDisplayLabel,
  authorViewFromDirectMessageConversation,
  formatCount,
  resolveProfilePictureSrc,
} from '@/shell/selectors';
import {
  selectPrimaryImageAttachment,
  selectVideoManifestAttachment,
  selectVideoPosterAttachment,
} from '@/shell/media';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import type {
  OpenAuthorDetail,
  OpenDirectMessagePane,
  OpenThread,
  Translate,
} from '@/shell/actions/shared';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;

type NotificationItemView = NotificationView & {
  actorLabel: string;
  actorPicture: string | null;
  contextLabel: string;
  kindLabel: string;
  previewText: string;
  receivedLabel: string;
  unread: boolean;
};

type MessagesWorkspaceProps = {
  t: Translate;
  locale: SupportedLocale;
  viewModels: Pick<
    ViewModels,
    | 'directMessageDraftViews'
    | 'selectedDirectMessagePeerLabel'
    | 'selectedDirectMessagePeerPicture'
    | 'selectedDirectMessageStatus'
    | 'selectedDirectMessageTimeline'
    | 'localDirectMessageAuthorPicture'
  >;
  openDirectMessageList: (mode?: 'push' | 'replace') => void;
  openDirectMessagePane: OpenDirectMessagePane;
  openAuthorDetail: OpenAuthorDetail;
  handleClearDirectMessage: (peerPubkey: string) => Promise<void>;
  handleDeleteDirectMessageMessage: (peerPubkey: string, messageId: string) => Promise<void>;
  handleDirectMessageAttachmentSelection: (event: ChangeEvent<HTMLInputElement>) => Promise<void>;
  handleRemoveDirectMessageDraftAttachment: (itemId: string) => void;
  handleSendDirectMessage: (event: FormEvent<HTMLFormElement>) => Promise<void>;
};

export function DesktopShellMessagesWorkspace({
  t,
  locale,
  viewModels,
  openDirectMessageList,
  openDirectMessagePane,
  openAuthorDetail,
  handleClearDirectMessage,
  handleDeleteDirectMessageMessage,
  handleDirectMessageAttachmentSelection,
  handleRemoveDirectMessageDraftAttachment,
  handleSendDirectMessage,
}: MessagesWorkspaceProps) {
  const {
    directMessageAttachmentInputKey,
    directMessageComposer,
    directMessageError,
    directMessageSending,
    directMessages,
    knownAuthorsByPubkey,
    localProfile,
    mediaObjectUrls,
    selectedDirectMessagePeerPubkey,
    syncStatus,
    unsupportedVideoManifests,
  } = useDesktopShellStore();
  const setDirectMessageComposer = useDesktopShellFieldSetter('directMessageComposer');
  const profileAuthorLabel = authorDisplayLabel(
    syncStatus.local_author_pubkey,
    localProfile?.display_name,
    localProfile?.name
  );

  return (
    <>
      <Card className='shell-workspace-card'>
        <div className='panel-header'>
          <div>
            <h3>Messages</h3>
            <small>{formatCount(directMessages.length)} conversations</small>
          </div>
          {selectedDirectMessagePeerPubkey ? (
            <Button variant='secondary' type='button' onClick={() => void openDirectMessageList('replace')}>
              All
            </Button>
          ) : null}
        </div>
        {directMessageError ? <Notice tone='destructive'>{directMessageError}</Notice> : null}
        {directMessages.length === 0 ? (
          <p className='empty'>No direct messages yet.</p>
        ) : (
          <ul className='post-list'>
            {directMessages.map((conversation) => {
              const label = authorDisplayLabel(
                conversation.peer_pubkey,
                conversation.peer_display_name,
                conversation.peer_name
              );
              const knownAuthor =
                knownAuthorsByPubkey[conversation.peer_pubkey] ??
                authorViewFromDirectMessageConversation(conversation);
              const picture = resolveProfilePictureSrc(knownAuthor, mediaObjectUrls);
              const selected = conversation.peer_pubkey === selectedDirectMessagePeerPubkey;
              return (
                <li key={conversation.peer_pubkey}>
                  <article className='post-card'>
                    <div className='post-meta'>
                      <AuthorIdentityButton
                        label={label}
                        picture={picture}
                        avatarTestId={`dm-conversation-avatar-${conversation.peer_pubkey}`}
                        onClick={() =>
                          void openAuthorDetail(conversation.peer_pubkey, {
                            historyMode: 'push',
                            preserveDirectMessageContext: true,
                            directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                          })
                        }
                      />
                      <span>
                        {conversation.last_message_at
                          ? formatLocalizedTime(conversation.last_message_at, locale)
                          : t('common:fallbacks.noEvents')}
                      </span>
                    </div>
                    <div className='post-body'>
                      <strong className='post-title'>
                        Latest: {conversation.last_message_preview ?? t('common:fallbacks.none')}
                      </strong>
                    </div>
                    <div className='post-actions'>
                      <Button
                        variant={selected ? 'primary' : 'secondary'}
                        type='button'
                        onClick={() => void openDirectMessagePane(conversation.peer_pubkey)}
                      >
                        Open
                      </Button>
                    </div>
                  </article>
                </li>
              );
            })}
          </ul>
        )}
      </Card>

      {selectedDirectMessagePeerPubkey ? (
        <>
          <Card className='shell-workspace-card'>
            <div className='shell-workspace-header'>
              <div className='shell-workspace-summary'>
                <AuthorIdentityButton
                  label={
                    viewModels.selectedDirectMessagePeerLabel ?? selectedDirectMessagePeerPubkey
                  }
                  picture={viewModels.selectedDirectMessagePeerPicture}
                  avatarSize='lg'
                  avatarTestId='dm-active-header-avatar'
                  className='relationship-badge'
                  onClick={() =>
                    void openAuthorDetail(selectedDirectMessagePeerPubkey, {
                      historyMode: 'push',
                      preserveDirectMessageContext: true,
                      directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                    })
                  }
                />
                {viewModels.selectedDirectMessageStatus ? (
                  <span className='relationship-badge relationship-badge-direct'>
                    {viewModels.selectedDirectMessageStatus.send_enabled
                      ? `peers ${formatCount(viewModels.selectedDirectMessageStatus.peer_count)}`
                      : 'send disabled'}
                  </span>
                ) : null}
              </div>
              <div className='post-actions'>
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() =>
                    void openDirectMessagePane(selectedDirectMessagePeerPubkey, {
                      historyMode: 'replace',
                    })
                  }
                >
                  {t('common:actions.refresh')}
                </Button>
                <Button
                  variant='secondary'
                  type='button'
                  disabled={viewModels.selectedDirectMessageTimeline.length === 0}
                  onClick={() => void handleClearDirectMessage(selectedDirectMessagePeerPubkey)}
                >
                  {t('common:actions.clear')}
                </Button>
              </div>
            </div>
          </Card>

          <Card className='shell-workspace-card'>
            {viewModels.selectedDirectMessageTimeline.length === 0 ? (
              <p className='empty'>No messages yet.</p>
            ) : (
              <ul className='post-list'>
                {viewModels.selectedDirectMessageTimeline.map((message) => {
                  const image = selectPrimaryImageAttachment(message.attachments);
                  const poster = selectVideoPosterAttachment(message.attachments);
                  const video = selectVideoManifestAttachment(message.attachments);
                  const imageSrc = image ? mediaObjectUrls[image.hash] ?? null : null;
                  const posterSrc = poster ? mediaObjectUrls[poster.hash] ?? null : null;
                  const videoSrc = video ? mediaObjectUrls[video.hash] ?? null : null;
                  const videoUnsupported = Boolean(video && unsupportedVideoManifests[video.hash]);
                  const authorPubkey = message.outgoing
                    ? syncStatus.local_author_pubkey
                    : selectedDirectMessagePeerPubkey;
                  const authorLabel = message.outgoing
                    ? profileAuthorLabel
                    : viewModels.selectedDirectMessagePeerLabel ?? selectedDirectMessagePeerPubkey;
                  const authorPicture = message.outgoing
                    ? viewModels.localDirectMessageAuthorPicture
                    : viewModels.selectedDirectMessagePeerPicture;
                  return (
                    <li key={message.message_id}>
                      <article className='post-card'>
                        <div className='post-meta'>
                          <AuthorIdentityButton
                            label={authorLabel}
                            picture={authorPicture}
                            avatarTestId={`dm-message-avatar-${message.message_id}`}
                            onClick={() =>
                              void openAuthorDetail(authorPubkey, {
                                historyMode: 'push',
                                preserveDirectMessageContext: true,
                                directMessagePeerPubkey: selectedDirectMessagePeerPubkey,
                              })
                            }
                          />
                          <span>{formatLocalizedTime(message.created_at, locale)}</span>
                          <span className='reply-chip'>
                            {message.delivered ? 'Delivered' : 'Pending'}
                          </span>
                        </div>
                        {message.text ? (
                          <div className='post-body'>
                            <strong className='post-title'>{message.text}</strong>
                          </div>
                        ) : null}
                        {image ? (
                          imageSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={imageSrc}
                                alt={t('common:media.imageAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingImage')}</small>
                          )
                        ) : null}
                        {video ? (
                          videoSrc && !videoUnsupported ? (
                            <video
                              className='post-card-video'
                              controls
                              playsInline
                              poster={posterSrc ?? undefined}
                              src={videoSrc}
                            />
                          ) : posterSrc ? (
                            <div className='draft-preview-frame'>
                              <img
                                className='draft-preview-image'
                                src={posterSrc}
                                alt={t('common:media.videoPosterAlt')}
                              />
                            </div>
                          ) : (
                            <small>{t('common:media.syncingPoster')}</small>
                          )
                        ) : null}
                        <div className='post-actions'>
                          <Button
                            variant='secondary'
                            type='button'
                            onClick={() =>
                              void handleDeleteDirectMessageMessage(
                                selectedDirectMessagePeerPubkey,
                                message.message_id
                              )
                            }
                          >
                            {t('common:actions.clear')}
                          </Button>
                        </div>
                      </article>
                    </li>
                  );
                })}
              </ul>
            )}
          </Card>

          <Card className='shell-workspace-card'>
            {viewModels.selectedDirectMessageStatus &&
            !viewModels.selectedDirectMessageStatus.send_enabled ? (
              <Notice tone='warning'>
                Direct message send is disabled until the relationship is mutual again.
              </Notice>
            ) : null}
            <form className='composer' onSubmit={(event) => void handleSendDirectMessage(event)}>
              <Textarea
                value={directMessageComposer}
                onChange={(event) => setDirectMessageComposer(event.target.value)}
                placeholder='Write a message'
                disabled={
                  directMessageSending || viewModels.selectedDirectMessageStatus?.send_enabled === false
                }
              />
              <Label className='file-field file-field-compact'>
                <span>{t('common:fallbacks.attachment')}</span>
                <Input
                  key={directMessageAttachmentInputKey}
                  aria-label={t('common:fallbacks.attachment')}
                  type='file'
                  accept='image/*,video/*'
                  disabled={
                    directMessageSending || viewModels.selectedDirectMessageStatus?.send_enabled === false
                  }
                  onChange={(event) => {
                    void handleDirectMessageAttachmentSelection(event);
                  }}
                />
              </Label>
              <ComposerDraftPreviewList
                items={viewModels.directMessageDraftViews}
                onRemove={handleRemoveDirectMessageDraftAttachment}
              />
              <div className='topic-diagnostic topic-diagnostic-secondary'>
                <span>
                  pending outbox {formatCount(viewModels.selectedDirectMessageStatus?.pending_outbox_count ?? 0)}
                </span>
              </div>
              <Button
                type='submit'
                disabled={
                  directMessageSending || viewModels.selectedDirectMessageStatus?.send_enabled === false
                }
              >
                {directMessageSending ? 'Sending...' : 'Send'}
              </Button>
            </form>
          </Card>
        </>
      ) : null}
    </>
  );
}

type NotificationsWorkspaceProps = {
  t: Translate;
  notificationItems: NotificationItemView[];
  onRefresh: () => void;
  handleOpenNotification: (notification: NotificationView) => Promise<void>;
};

export function DesktopShellNotificationsWorkspace({
  t,
  notificationItems,
  onRefresh,
  handleOpenNotification,
}: NotificationsWorkspaceProps) {
  const {
    notifications,
    notificationAutoReadError,
    notificationPanelState,
    notificationStatus,
  } = useDesktopShellStore();

  return (
    <>
      <Card className='shell-workspace-card'>
        <div className='shell-workspace-header'>
          <div>
            <h3>{t('shell:notifications.title')}</h3>
            <small>
              {t('shell:notifications.summary', {
                count: notifications.length,
                unread: notificationStatus.unread_count,
              })}
            </small>
          </div>
          <Button variant='secondary' type='button' onClick={onRefresh}>
            {t('common:actions.refresh')}
          </Button>
        </div>
        {notificationPanelState.status === 'loading' ? (
          <Notice>{t('shell:notifications.loading')}</Notice>
        ) : null}
        {notificationPanelState.status === 'error' && notificationPanelState.error ? (
          <Notice tone='destructive'>{notificationPanelState.error}</Notice>
        ) : null}
        {notificationAutoReadError ? <Notice tone='warning'>{notificationAutoReadError}</Notice> : null}
      </Card>

      <Card className='shell-workspace-card'>
        {notificationPanelState.status === 'ready' && notificationItems.length === 0 ? (
          <p className='empty-state'>{t('shell:notifications.empty')}</p>
        ) : null}
        {notificationItems.length > 0 ? (
          <ul className='notification-list' aria-label={t('shell:notifications.title')}>
            {notificationItems.map((notification) => (
              <li key={notification.notification_id}>
                <button
                  className='notification-item'
                  data-unread={notification.unread}
                  type='button'
                  onClick={() => void handleOpenNotification(notification)}
                >
                  <div className='notification-item-header'>
                    <div className='notification-item-author'>
                      <AuthorAvatar
                        label={notification.actorLabel}
                        picture={notification.actorPicture}
                        testId={`notification-avatar-${notification.notification_id}`}
                      />
                      <div className='notification-item-copy'>
                        <span className='notification-item-author-label'>
                          {notification.actorLabel}
                        </span>
                        <div className='notification-item-badges'>
                          <Badge tone={notification.unread ? 'accent' : 'neutral'}>
                            {notification.kindLabel}
                          </Badge>
                          {notification.unread ? (
                            <Badge tone='warning'>{t('shell:notifications.unread')}</Badge>
                          ) : null}
                        </div>
                      </div>
                    </div>
                    <span className='notification-item-time'>{notification.receivedLabel}</span>
                  </div>
                  <div className='notification-item-body'>
                    <p className='notification-item-preview'>{notification.previewText}</p>
                    <small className='notification-item-context'>{notification.contextLabel}</small>
                  </div>
                </button>
              </li>
            ))}
          </ul>
        ) : null}
      </Card>
    </>
  );
}

type DetailPaneStackProps = {
  t: Translate;
  activeTopic: string;
  viewModels: Pick<
    ViewModels,
    | 'authorDetailView'
    | 'selectedAuthorTimelinePostViews'
    | 'threadPanelState'
    | 'threadPostViews'
  >;
  closeAuthorPane: () => void;
  closeThreadPane: () => void;
  loadMoreThread: (topic: string, threadId: string) => Promise<void>;
  loadReactionCatalogData: () => Promise<void>;
  openAuthorDetail: OpenAuthorDetail;
  openDirectMessagePane: OpenDirectMessagePane;
  openThread: OpenThread;
  beginReply: (post: PostView) => void;
  handleSimpleRepost: (post: PostView) => Promise<void>;
  beginQuoteRepost: (post: PostView) => void;
  handleRetryLocalPost: (post: PostView) => void;
  handleRestoreLocalPost: (post: PostView) => void;
  handleToggleReaction: (post: PostView, reactionKey: ReactionKeyInput) => Promise<void>;
  handleBookmarkCustomReaction: (
    asset: Parameters<import('@/lib/api').DesktopApi['bookmarkCustomReaction']>[0]
  ) => Promise<void>;
  handleActivateReference: (reference: InternalSmartReference) => Promise<void>;
  handleCopyPostLink: (link: string) => void;
  handleRelationshipAction: (authorPubkey: string, following: boolean) => Promise<void>;
  handleMuteAction: (authorPubkey: string, muted: boolean) => Promise<void>;
  handleOpenOriginalTopic: (topicId: string) => Promise<void>;
};

export function DesktopShellDetailPaneStack({
  t,
  activeTopic,
  viewModels,
  closeAuthorPane,
  closeThreadPane,
  loadMoreThread,
  loadReactionCatalogData,
  openAuthorDetail,
  openDirectMessagePane,
  openThread,
  beginReply,
  handleSimpleRepost,
  beginQuoteRepost,
  handleRetryLocalPost,
  handleRestoreLocalPost,
  handleToggleReaction,
  handleBookmarkCustomReaction,
  handleActivateReference,
  handleCopyPostLink,
  handleRelationshipAction,
  handleMuteAction,
  handleOpenOriginalTopic,
}: DetailPaneStackProps) {
  const {
    bookmarkedReactionAssets,
    focusedObjectId,
    mediaObjectUrls,
    ownedReactionAssets,
    recentReactions,
    selectedAuthor,
    selectedAuthorPubkey,
    selectedThread,
    syncStatus,
    threadLoadingMoreById,
    threadNextCursorById,
  } = useDesktopShellStore();
  const selectedThreadHasMore = selectedThread ? Boolean(threadNextCursorById[selectedThread]) : false;
  const selectedThreadLoadingMore = selectedThread
    ? (threadLoadingMoreById[selectedThread] ?? false)
    : false;

  return (
    <>
      {selectedThread ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-thread`}
          title={t('shell:context.thread')}
          summary={viewModels.threadPanelState.summary}
          showBackdrop={!selectedAuthorPubkey}
          stackIndex={0}
          onClose={closeThreadPane}
        >
          <ThreadPanel
            state={viewModels.threadPanelState}
            posts={viewModels.threadPostViews}
            hasMore={selectedThreadHasMore}
            loadingMore={selectedThreadLoadingMore}
            onLoadMore={() => void loadMoreThread(activeTopic, selectedThread)}
            onOpenAuthor={(authorPubkey) =>
              void openAuthorDetail(authorPubkey, {
                fromThread: true,
                threadId: selectedThread,
              })
            }
            onOpenThread={(threadId) => void openThread(threadId)}
            onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
            onReply={beginReply}
            onRepost={(post) => void handleSimpleRepost(post)}
            onQuoteRepost={beginQuoteRepost}
            onRetryLocalPost={handleRetryLocalPost}
            onRestoreLocalPost={handleRestoreLocalPost}
            localAuthorPubkey={syncStatus.local_author_pubkey}
            mediaObjectUrls={mediaObjectUrls}
            ownedReactionAssets={ownedReactionAssets}
            bookmarkedReactionAssets={bookmarkedReactionAssets}
            recentReactions={recentReactions}
            onToggleReaction={(post, reactionKey) => void handleToggleReaction(post, reactionKey)}
            onBookmarkCustomReaction={(asset) => void handleBookmarkCustomReaction(asset)}
            onReactionPickerOpen={() => void loadReactionCatalogData()}
            onActivateReference={(reference) => void handleActivateReference(reference)}
            onCopyPostLink={handleCopyPostLink}
            focusedPostObjectId={focusedObjectId}
          />
        </ContextPane>
      ) : null}
      {selectedAuthorPubkey ? (
        <ContextPane
          paneId={`${SHELL_CONTEXT_ID}-author`}
          title={t('shell:context.author')}
          summary={
            selectedAuthor
              ? viewModels.authorDetailView.displayLabel
              : t('common:fallbacks.selectAuthor')
          }
          showBackdrop={true}
          stackIndex={selectedThread ? 1 : 0}
          onClose={closeAuthorPane}
        >
          <div className='shell-main-stack'>
            <AuthorDetailCard
              view={viewModels.authorDetailView}
              localAuthorPubkey={syncStatus.local_author_pubkey}
              onToggleRelationship={(authorPubkey, following) =>
                void handleRelationshipAction(authorPubkey, following)
              }
              onToggleMute={(authorPubkey, muted) => void handleMuteAction(authorPubkey, muted)}
              onOpenDirectMessage={(authorPubkey) => void openDirectMessagePane(authorPubkey)}
            />
            <Card className='shell-workspace-card'>
              <TimelineFeed
                posts={viewModels.selectedAuthorTimelinePostViews}
                emptyCopy={t('profile:feed.noAuthorPosts')}
                onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                onOpenThread={(threadId) => void openThread(threadId)}
                onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                onReply={beginReply}
                readOnly={true}
                onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
                onActivateReference={(reference) => void handleActivateReference(reference)}
                onCopyPostLink={handleCopyPostLink}
              />
            </Card>
          </div>
        </ContextPane>
      ) : null}
    </>
  );
}
