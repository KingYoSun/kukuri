import { type FormEvent, type ReactNode, useMemo } from 'react';
import { Link2 } from 'lucide-react';

import { TimelineWorkspaceHeader } from '@/components/core/TimelineWorkspaceHeader';
import { TimelineFeed } from '@/components/core/TimelineFeed';
import { MetaverseRoomPanel } from '@/components/extended/MetaverseRoomPanel';
import { ProfileConnectionsPanel } from '@/components/extended/ProfileConnectionsPanel';
import { ProfileEditorPanel } from '@/components/extended/ProfileEditorPanel';
import { ProfileOverviewPanel } from '@/components/extended/ProfileOverviewPanel';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
import { SmartReferenceText } from '@/components/core/SmartReferenceText';
import type { PrimarySection, ProfileConnectionsView } from '@/components/shell/types';

import type {
  DesktopApi,
  GameRoomStatus,
  PostView,
  ReactionKeyInput,
} from '@/lib/api';
import { formatLocalizedTime } from '@/i18n/format';
import type { SupportedLocale } from '@/i18n';
import { buildGameLink, buildLiveLink, type InternalSmartReference } from '@/lib/internalLinks';
import {
  timelineScopeStorageKey,
  type GameEditorDraft,
  useDesktopShellFieldSetter,
  useDesktopShellStore,
} from '@/shell/store';
import {
  formatCount,
  localizeAudienceLabel,
  resolveProfilePictureSrc,
  translateGameStatus,
  translateLiveStatus,
} from '@/shell/selectors';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import type { OpenAuthorDetail, OpenThread, Translate } from '@/shell/actions/shared';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;

type DesktopShellPrimaryWorkspaceProps = {
  t: Translate;
  api: DesktopApi;
  locale: SupportedLocale;
  routeSection: PrimarySection;
  profileAuthorLabel: string;
  profileAvatarInputKey: number;
  messagesWorkspace: ReactNode;
  notificationsWorkspace: ReactNode;
  viewModels: Pick<
    ViewModels,
    | 'activeComposeAudienceLabel'
    | 'activeComposeChannel'
    | 'activeGamePanelState'
    | 'activeGameRooms'
    | 'activeLivePanelState'
    | 'activeSocialConnectionViews'
    | 'activeTimelinePostViews'
    | 'activeTimelineScope'
    | 'bookmarkedTimelinePostViews'
    | 'composerSourcePreview'
    | 'gameDraftViews'
    | 'liveSessionListItems'
    | 'primarySectionItems'
    | 'profileEditorFields'
    | 'profileEditorHasPicture'
    | 'profileEditorPictureSrc'
    | 'profileTimelinePostViews'
    | 'selectedAuthorTimelinePostViews'
    | 'timelineViewItems'
  >;
  setPrimarySectionRef: (section: PrimarySection) => (node: HTMLElement | null) => void;
  focusPrimarySection: (section: PrimarySection) => void;
  focusTimelineView: (view: 'feed' | 'bookmarks') => void;
  loadReactionCatalogData: () => Promise<void>;
  refreshTimelineFeed: (topic: string, currentThread: string | null) => Promise<void>;
  refreshCurrentTopic: () => Promise<void>;
  loadMoreTimeline: (topic: string) => Promise<void>;
  openAuthorDetail: OpenAuthorDetail;
  openThread: OpenThread;
  beginReply: (post: PostView) => void;
  handleSimpleRepost: (post: PostView) => Promise<void>;
  beginQuoteRepost: (post: PostView) => void;
  handleRetryLocalPost: (post: PostView) => void;
  handleRestoreLocalPost: (post: PostView) => void;
  handleToggleReaction: (post: PostView, reactionKey: ReactionKeyInput) => Promise<void>;
  handleBookmarkCustomReaction: (
    asset: Parameters<DesktopApi['bookmarkCustomReaction']>[0]
  ) => Promise<void>;
  handleToggleBookmarkedPost: (post: PostView) => Promise<void>;
  handleActivateReference: (reference: InternalSmartReference) => Promise<void>;
  handleCopyInternalLink: (link: string) => void;
  handleJoinLiveSession: (sessionId: string) => Promise<void>;
  handleLeaveLiveSession: (sessionId: string) => Promise<void>;
  handleEndLiveSession: (sessionId: string) => Promise<void>;
  updateGameDraft: (roomId: string, update: (draft: GameEditorDraft) => GameEditorDraft) => void;
  handleUpdateGameRoom: (roomId: string) => Promise<void>;
  openProfileOverview: () => void;
  openProfileEditor: () => void;
  openProfileConnections: (view: ProfileConnectionsView) => void;
  handleProfileFieldChange: (field: 'displayName' | 'name' | 'about', value: string) => void;
  onProfilePictureSelect: (file: File) => void;
  handleClearProfileAvatar: () => void;
  handleSaveProfile: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  resetProfileDraft: () => void;
  handleRelationshipAction: (authorPubkey: string, following: boolean) => Promise<void>;
  handleMuteAction: (authorPubkey: string, muted: boolean) => Promise<void>;
  handleOpenOriginalTopic: (topicId: string) => Promise<void>;
};

export function DesktopShellPrimaryWorkspace({
  t,
  api,
  locale,
  routeSection,
  profileAuthorLabel,
  profileAvatarInputKey,
  messagesWorkspace,
  notificationsWorkspace,
  viewModels,
  setPrimarySectionRef,
  focusPrimarySection,
  focusTimelineView,
  loadReactionCatalogData,
  refreshTimelineFeed,
  refreshCurrentTopic,
  loadMoreTimeline,
  openAuthorDetail,
  openThread,
  beginReply,
  handleSimpleRepost,
  beginQuoteRepost,
  handleRetryLocalPost,
  handleRestoreLocalPost,
  handleToggleReaction,
  handleBookmarkCustomReaction,
  handleToggleBookmarkedPost,
  handleActivateReference,
  handleCopyInternalLink,
  handleJoinLiveSession,
  handleLeaveLiveSession,
  handleEndLiveSession,
  updateGameDraft,
  handleUpdateGameRoom,
  openProfileOverview,
  openProfileEditor,
  openProfileConnections,
  handleProfileFieldChange,
  onProfilePictureSelect,
  handleClearProfileAvatar,
  handleSaveProfile,
  resetProfileDraft,
  handleRelationshipAction,
  handleMuteAction,
  handleOpenOriginalTopic,
}: DesktopShellPrimaryWorkspaceProps) {
  const {
    activeTopic,
    bookmarkedPosts,
    bookmarkedReactionAssets,
    composerError,
    gameError,
    gameSavingByRoomId,
    knownAuthorsByPubkey,
    liveError,
    localProfile,
    mediaObjectUrls,
    communityNodeStatuses,
    ownedReactionAssets,
    pendingTimelineCountsByKey,
    profileDirty,
    profileError,
    profilePanelState,
    profileSaving,
    recentReactions,
    selectedGameRoomId,
    selectedLiveSessionId,
    selectedThread,
    shellChromeState,
    socialConnections,
    socialConnectionsPanelState,
    syncStatus,
    timelineLoadingMoreByKey,
    timelineNextCursorByKey,
  } = useDesktopShellStore();
  const setShellChromeState = useDesktopShellFieldSetter('shellChromeState');
  const bookmarkedPostIds = useMemo(
    () => new Set(bookmarkedPosts.map((item) => item.post.object_id)),
    [bookmarkedPosts]
  );
  const activeTimelineKey = timelineScopeStorageKey(activeTopic, viewModels.activeTimelineScope);
  const activeTimelinePendingCount = pendingTimelineCountsByKey[activeTimelineKey] ?? 0;
  const activeTimelineHasMore = Boolean(timelineNextCursorByKey[activeTimelineKey]);
  const activeTimelineLoadingMore = timelineLoadingMoreByKey[activeTimelineKey] ?? false;
  const metaverseRooms = useMemo(
    () => viewModels.activeGameRooms.filter((room) => room.room_kind === 'metaverse_room'),
    [viewModels.activeGameRooms]
  );
  const scoreGameRooms = useMemo(
    () => viewModels.activeGameRooms.filter((room) => room.room_kind !== 'metaverse_room'),
    [viewModels.activeGameRooms]
  );
  const profileMode = shellChromeState.profileMode;
  const profileConnectionsView = shellChromeState.profileConnectionsView;
  const unavailableCommunityNodeCount = communityNodeStatuses.filter(
    (status) => Boolean(status.last_error)
  ).length;
  const showCommunityNodeUnavailableNotice =
    communityNodeStatuses.length > 0 && unavailableCommunityNodeCount > 0;

  return (
    <div className='shell-main-stack'>
      {showCommunityNodeUnavailableNotice ? (
        <Notice
          tone='warning'
          className='flex flex-col gap-3 md:flex-row md:items-start md:justify-between'
          data-testid='community-node-unavailable-notice'
        >
          <div className='space-y-1'>
            <p className='font-semibold'>{t('shell:workspace.communityNodeUnavailableTitle')}</p>
            <p>{t('shell:workspace.communityNodeUnavailableBody')}</p>
          </div>
          <Button
            variant='secondary'
            type='button'
            onClick={() =>
              setShellChromeState((current) => ({
                ...current,
                settingsOpen: true,
                activeSettingsSection: 'community-node',
              }))
            }
          >
            {t('shell:workspace.communityNodeUnavailableAction')}
          </Button>
        </Notice>
      ) : null}
      {shellChromeState.activePrimarySection !== 'notifications' ? (
        <Card className='shell-workspace-card shell-workspace-header-card'>
          <TimelineWorkspaceHeader
            activeSection={shellChromeState.activePrimarySection}
            items={viewModels.primarySectionItems}
            onSelectSection={focusPrimarySection}
          />
        </Card>
      ) : null}

      <section
        className='shell-section'
        ref={setPrimarySectionRef(shellChromeState.activePrimarySection)}
        tabIndex={-1}
        onFocusCapture={() =>
          setShellChromeState((current) => ({
            ...current,
            activePrimarySection: routeSection,
          }))
        }
      >
        {shellChromeState.activePrimarySection === 'timeline' ? (
          <>
            <Card className='shell-workspace-card'>
              <div className='shell-workspace-header'>
                <div className='shell-workspace-summary'>
                  <div
                    className='shell-workspace-tabs'
                    role='tablist'
                    aria-label={t('shell:workspace.timelineViews')}
                  >
                    {viewModels.timelineViewItems.map((item) => (
                      <button
                        key={item.id}
                        className={`shell-tab${
                          shellChromeState.timelineView === item.id ? ' shell-tab-active' : ''
                        }`}
                        role='tab'
                        type='button'
                        aria-selected={shellChromeState.timelineView === item.id}
                        onClick={() => focusTimelineView(item.id)}
                      >
                        {item.label}
                      </button>
                    ))}
                  </div>
                  {shellChromeState.timelineView === 'bookmarks' ? (
                    <span className='relationship-badge'>
                      {t('shell:workspace.savedCount', {
                        count: viewModels.bookmarkedTimelinePostViews.length,
                      })}
                    </span>
                  ) : null}
                </div>
              </div>
              {composerError ? <Notice tone='destructive'>{composerError}</Notice> : null}
            </Card>
            <Card className='shell-workspace-card'>
              {shellChromeState.timelineView === 'feed' ? (
                <TimelineFeed
                  posts={viewModels.activeTimelinePostViews}
                  emptyCopy={t('shell:workspace.noPosts')}
                  onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
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
                  showBookmarkAction={true}
                  bookmarkedPostIds={bookmarkedPostIds}
                  onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                  onActivateReference={(reference) => void handleActivateReference(reference)}
                  onCopyPostLink={handleCopyInternalLink}
                  hasMore={activeTimelineHasMore}
                  loadingMore={activeTimelineLoadingMore}
                  onLoadMore={() => void loadMoreTimeline(activeTopic)}
                  pendingCount={activeTimelinePendingCount}
                  onApplyPending={() => void refreshTimelineFeed(activeTopic, selectedThread)}
                />
              ) : (
                <TimelineFeed
                  posts={viewModels.bookmarkedTimelinePostViews}
                  emptyCopy={t('shell:workspace.noBookmarks')}
                  onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
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
                  showBookmarkAction={true}
                  bookmarkedPostIds={bookmarkedPostIds}
                  onToggleBookmark={(post) => void handleToggleBookmarkedPost(post)}
                  onActivateReference={(reference) => void handleActivateReference(reference)}
                  onCopyPostLink={handleCopyInternalLink}
                />
              )}
            </Card>
          </>
        ) : null}

        {shellChromeState.activePrimarySection === 'live' ? (
          <>
            <Card className='shell-workspace-card'>
              <div className='panel-header'>
                <div>
                  <h3>{t('live:title')}</h3>
                  <small>{t('live:summary', { count: viewModels.liveSessionListItems.length })}</small>
                </div>
              </div>
              {viewModels.activeLivePanelState.status === 'loading' ? (
                <Notice>{t('live:loading')}</Notice>
              ) : null}
              {viewModels.activeLivePanelState.status === 'error' &&
              (liveError ?? viewModels.activeLivePanelState.error) ? (
                <Notice tone='destructive'>{liveError ?? viewModels.activeLivePanelState.error}</Notice>
              ) : null}
            </Card>
            <Card className='shell-workspace-card'>
              {viewModels.liveSessionListItems.length === 0 &&
              viewModels.activeLivePanelState.status === 'ready' ? (
                <p className='empty-state'>{t('live:empty')}</p>
              ) : null}
              <ul className='post-list'>
                {viewModels.liveSessionListItems.map(({ session, isOwner, pending }) => (
                  <li key={session.session_id}>
                    <article
                      className={`post-card${
                        selectedLiveSessionId === session.session_id ? ' post-card-targeted' : ''
                      }`}
                      aria-busy={pending}
                      data-live-session-id={session.session_id}
                      tabIndex={selectedLiveSessionId === session.session_id ? -1 : undefined}
                    >
                      <div className='post-meta'>
                        <span>{session.title}</span>
                        <span>{translateLiveStatus(session.status)}</span>
                        <span className='reply-chip'>
                          {localizeAudienceLabel(session.audience_label)}
                        </span>
                      </div>
                      <div className='post-body'>
                        <strong className='post-title post-copy-wrap'>
                          <SmartReferenceText
                            text={session.description || t('common:fallbacks.noDescription')}
                            className='post-copy-wrap'
                            onActivateReference={(reference) => void handleActivateReference(reference)}
                          />
                        </strong>
                      </div>
                      <small>{session.session_id}</small>
                      <div className='topic-diagnostic topic-diagnostic-secondary'>
                        <span>{t('common:labels.viewers')}: {formatCount(session.viewer_count)}</span>
                        <span>
                          {t('common:labels.started')}: {formatLocalizedTime(session.started_at, locale)}
                        </span>
                      </div>
                      {session.ended_at ? (
                        <div className='topic-diagnostic topic-diagnostic-secondary'>
                          <span>
                            {t('common:labels.ended')}: {formatLocalizedTime(session.ended_at, locale)}
                          </span>
                        </div>
                      ) : null}
                      <div className='post-actions'>
                        {session.joined_by_me ? (
                          <Button
                            variant='secondary'
                            type='button'
                            disabled={pending}
                            onClick={() => void handleLeaveLiveSession(session.session_id)}
                          >
                            {t('common:actions.leave')}
                          </Button>
                        ) : (
                          <Button
                            variant='secondary'
                            type='button'
                            disabled={pending || session.status === 'Ended'}
                            onClick={() => void handleJoinLiveSession(session.session_id)}
                          >
                            {t('common:actions.join')}
                          </Button>
                        )}
                        {isOwner ? (
                          <Button
                            variant='secondary'
                            type='button'
                            disabled={pending || session.status === 'Ended'}
                            onClick={() => void handleEndLiveSession(session.session_id)}
                          >
                            {t('common:actions.end')}
                          </Button>
                        ) : null}
                        <Button
                          variant='secondary'
                          size='icon'
                          className='post-action-button'
                          type='button'
                          aria-label={t('common:actions.copyLink')}
                          onClick={() =>
                            handleCopyInternalLink(
                              buildLiveLink(activeTopic, session.session_id, session.channel_id ?? null)
                            )
                          }
                        >
                          <Link2 className='size-4' aria-hidden='true' />
                        </Button>
                      </div>
                    </article>
                  </li>
                ))}
              </ul>
            </Card>
          </>
        ) : null}

        {shellChromeState.activePrimarySection === 'game' ? (
          <>
            <Card className='shell-workspace-card'>
              <div className='panel-header'>
                <div>
                  <h3>{t('game:title')}</h3>
                  <small>{t('game:summary', { count: scoreGameRooms.length })}</small>
                </div>
              </div>
              {viewModels.activeGamePanelState.status === 'loading' ? (
                <Notice>{t('game:loading')}</Notice>
              ) : null}
              {viewModels.activeGamePanelState.status === 'error' &&
              (gameError ?? viewModels.activeGamePanelState.error) ? (
                <Notice tone='destructive'>{gameError ?? viewModels.activeGamePanelState.error}</Notice>
              ) : null}
            </Card>
            <Card className='shell-workspace-card'>
              {scoreGameRooms.length === 0 &&
              viewModels.activeGamePanelState.status === 'ready' ? (
                <p className='empty-state'>{t('game:empty')}</p>
              ) : null}
              <ul className='post-list'>
                {scoreGameRooms.map((room) => {
                  const draft = viewModels.gameDraftViews[room.room_id];
                  const isOwner = room.host_pubkey === syncStatus.local_author_pubkey;
                  const pending = Boolean(gameSavingByRoomId[room.room_id]);

                  return (
                    <li key={room.room_id}>
                      <article
                        className={`post-card${
                          selectedGameRoomId === room.room_id ? ' post-card-targeted' : ''
                        }`}
                        aria-busy={pending}
                        data-game-room-id={room.room_id}
                        tabIndex={selectedGameRoomId === room.room_id ? -1 : undefined}
                      >
                        <div className='post-meta'>
                          <span>{room.title}</span>
                          <span>{translateGameStatus(room.status)}</span>
                          <span className='reply-chip'>
                            {localizeAudienceLabel(room.audience_label)}
                          </span>
                        </div>
                        <div className='post-body'>
                          <strong className='post-title post-copy-wrap'>
                            <SmartReferenceText
                              text={room.description || t('common:fallbacks.noDescription')}
                              className='post-copy-wrap'
                              onActivateReference={(reference) => void handleActivateReference(reference)}
                            />
                          </strong>
                        </div>
                        <small>{room.room_id}</small>
                        <div className='topic-diagnostic topic-diagnostic-secondary'>
                          <span>
                            {t('common:labels.phase')}: {room.phase_label ?? t('common:fallbacks.none')}
                          </span>
                          <span>
                            {t('common:labels.updated')}: {formatLocalizedTime(room.updated_at, locale)}
                          </span>
                        </div>
                        <ul className='draft-attachment-list'>
                          {room.scores.map((score) => (
                            <li key={score.participant_id} className='draft-attachment-item score-row'>
                              <div className='draft-attachment-content'>
                                <strong>{score.label}</strong>
                              </div>
                              {isOwner ? (
                                <Input
                                  aria-label={`${room.room_id}-${score.label}-score`}
                                  value={draft?.scores[score.participant_id] ?? String(score.score)}
                                  disabled={pending}
                                  onChange={(event) =>
                                    updateGameDraft(room.room_id, (current) => ({
                                      ...current,
                                      scores: {
                                        ...current.scores,
                                        [score.participant_id]: event.target.value,
                                      },
                                    }))
                                  }
                                />
                              ) : (
                                <span>{score.score}</span>
                              )}
                            </li>
                          ))}
                        </ul>
                        {isOwner && draft ? (
                          <div className='composer composer-compact'>
                            <Label>
                              <span>{t('game:fields.status')}</span>
                              <Select
                                aria-label={`${room.room_id}-status`}
                                value={draft.status}
                                disabled={pending}
                                onChange={(event) =>
                                  updateGameDraft(room.room_id, (current) => ({
                                    ...current,
                                    status: event.target.value as GameRoomStatus,
                                  }))
                                }
                              >
                                <option value='Waiting'>{t('game:statuses.Waiting')}</option>
                                <option value='Running'>{t('game:statuses.Running')}</option>
                                <option value='Paused'>{t('game:statuses.Paused')}</option>
                                <option value='Ended'>{t('game:statuses.Ended')}</option>
                              </Select>
                            </Label>
                            <Label>
                              <span>{t('game:fields.phase')}</span>
                              <Input
                                aria-label={`${room.room_id}-phase`}
                                value={draft.phaseLabel}
                                disabled={pending}
                                onChange={(event) =>
                                  updateGameDraft(room.room_id, (current) => ({
                                    ...current,
                                    phase_label: event.target.value,
                                  }))
                                }
                              />
                            </Label>
                            <Button
                              variant='secondary'
                              type='button'
                              disabled={pending}
                              onClick={() => void handleUpdateGameRoom(room.room_id)}
                            >
                              {t('game:actions.saveRoom')}
                            </Button>
                          </div>
                        ) : null}
                        <div className='post-actions'>
                          <Button
                            variant='secondary'
                            size='icon'
                            className='post-action-button'
                            type='button'
                            aria-label={t('common:actions.copyLink')}
                            onClick={() =>
                              handleCopyInternalLink(
                                buildGameLink(activeTopic, room.room_id, room.channel_id ?? null)
                              )
                            }
                          >
                            <Link2 className='size-4' aria-hidden='true' />
                          </Button>
                        </div>
                      </article>
                    </li>
                  );
                })}
              </ul>
            </Card>
            <MetaverseRoomPanel
              api={api}
              activeTopic={activeTopic}
              activeComposeChannel={viewModels.activeComposeChannel}
              rooms={metaverseRooms}
              syncStatus={syncStatus}
              locale={locale}
              localProfile={localProfile}
              knownAuthorsByPubkey={knownAuthorsByPubkey}
              mediaObjectUrls={mediaObjectUrls}
              onRefresh={refreshCurrentTopic}
            />
          </>
        ) : null}

        {shellChromeState.activePrimarySection === 'notifications' ? notificationsWorkspace : null}

        {shellChromeState.activePrimarySection === 'messages' ? messagesWorkspace : null}

        {shellChromeState.activePrimarySection === 'profile' ? (
          <>
            {profileMode === 'edit' ? (
              <ProfileEditorPanel
                authorLabel={profileAuthorLabel}
                status={profilePanelState.status}
                saving={profileSaving}
                dirty={profileDirty}
                error={profileError ?? profilePanelState.error}
                fields={viewModels.profileEditorFields}
                picturePreviewSrc={viewModels.profileEditorPictureSrc}
                hasPicture={viewModels.profileEditorHasPicture}
                pictureInputKey={profileAvatarInputKey}
                onFieldChange={handleProfileFieldChange}
                onPictureSelect={(event) => {
                  const file = event.target.files?.[0] ?? null;
                  if (file) {
                    onProfilePictureSelect(file);
                  }
                }}
                onPictureClear={handleClearProfileAvatar}
                onBack={openProfileOverview}
                onSave={handleSaveProfile}
                onReset={resetProfileDraft}
              />
            ) : profileMode === 'connections' ? (
              <ProfileConnectionsPanel
                activeView={profileConnectionsView}
                items={viewModels.activeSocialConnectionViews}
                localAuthorPubkey={syncStatus.local_author_pubkey}
                status={socialConnectionsPanelState.status}
                error={socialConnectionsPanelState.error}
                onSelectView={openProfileConnections}
                onToggleRelationship={(authorPubkey, following) =>
                  void handleRelationshipAction(authorPubkey, following)
                }
                onToggleMute={(authorPubkey, muted) => void handleMuteAction(authorPubkey, muted)}
                onBack={openProfileOverview}
              />
            ) : (
              <ProfileOverviewPanel
                authorLabel={profileAuthorLabel}
                about={localProfile?.about ?? null}
                picture={resolveProfilePictureSrc(localProfile, mediaObjectUrls)}
                status={profilePanelState.status}
                error={profileError ?? profilePanelState.error}
                postCount={viewModels.profileTimelinePostViews.length}
                followingCount={socialConnections.following.length}
                followedCount={socialConnections.followed.length}
                mutedCount={socialConnections.muted.length}
                onEdit={openProfileEditor}
                onOpenFollowing={() => openProfileConnections('following')}
                onOpenFollowed={() => openProfileConnections('followed')}
                onOpenMuted={() => openProfileConnections('muted')}
              />
            )}
            {profileMode !== 'connections' ? (
              <Card className='shell-workspace-card'>
                <TimelineFeed
                  posts={viewModels.profileTimelinePostViews}
                  emptyCopy={t('profile:feed.noOwnPosts')}
                  onOpenAuthor={(authorPubkey) => void openAuthorDetail(authorPubkey)}
                  onOpenThread={(threadId) => void openThread(threadId)}
                  onOpenThreadInTopic={(threadId, topicId) => void openThread(threadId, { topic: topicId })}
                  onReply={beginReply}
                  readOnly={true}
                  onOpenOriginalTopic={(topicId) => void handleOpenOriginalTopic(topicId)}
                  onActivateReference={(reference) => void handleActivateReference(reference)}
                  onCopyPostLink={handleCopyInternalLink}
                />
              </Card>
            ) : null}
          </>
        ) : null}
      </section>
    </div>
  );
}
