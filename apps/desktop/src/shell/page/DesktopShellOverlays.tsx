import { Plus } from 'lucide-react';

import { ComposerPanel } from '@/components/core/ComposerPanel';
import {
  PrivateChannelPanel,
  PrivateChannelSettingsPanel,
} from '@/components/extended/PrivateChannelPanel';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { ImageCropDialog } from '@/components/ui/ImageCropDialog';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip';

import { authorDisplayLabel } from '@/shell/presentation';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import type { Translate } from '@/shell/actions/shared';
import type { useShellDialogs } from '@/shell/page/useShellDialogs';
import type { useSharePreview } from '@/shell/page/useSharePreview';
import type { useDesktopShellActions } from '@/shell/useDesktopShellActions';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';
import { useShallow } from 'zustand/react/shallow';
import { topicDisplayName } from '@/lib/topicId';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;
type OverlayActions = Pick<
  ReturnType<typeof useDesktopShellActions>,
  | 'clearReply'
  | 'clearRepost'
  | 'handleAttachmentSelection'
  | 'handleCreateGameRoom'
  | 'handleCreateLiveSession'
  | 'handleCreatePrivateChannel'
  | 'handleJoinChannelAccess'
  | 'handleLeavePrivateChannel'
  | 'handleProfileAvatarFile'
  | 'handlePublish'
  | 'handleRemoveDraftAttachment'
  | 'handleShareChannelAccess'
  | 'openFloatingActionDialog'
>;
type ShellDialogs = Pick<
  ReturnType<typeof useShellDialogs>,
  | 'channelDialogOpen'
  | 'channelSettingsDialogOpen'
  | 'composeDialogOpen'
  | 'confirmLeaveChannel'
  | 'gameCreateDialogOpen'
  | 'leaveChannelDialogOpen'
  | 'liveCreateDialogOpen'
  | 'profileAvatarCropFile'
  | 'profileAvatarCropOpen'
  | 'setChannelDialogOpen'
  | 'setChannelSettingsDialogOpen'
  | 'setComposeDialogOpen'
  | 'setGameCreateDialogOpen'
  | 'setLeaveChannelDialogOpen'
  | 'setLiveCreateDialogOpen'
  | 'setProfileAvatarCropFile'
  | 'setProfileAvatarCropOpen'
>;
type SharePreview = Pick<
  ReturnType<typeof useSharePreview>,
  | 'confirmImport'
  | 'data'
  | 'error'
  | 'handleOpenChange'
  | 'importPending'
  | 'loading'
  | 'open'
  | 'token'
>;

type DesktopShellOverlaysProps = {
  actions: OverlayActions;
  dialogs: ShellDialogs;
  t: Translate;
  viewModels: Pick<
    ViewModels,
    | 'activeComposeAudienceLabel'
    | 'activeChannelPanelState'
    | 'channelAudienceOptions'
    | 'composerDraftViews'
    | 'composerSourcePreview'
    | 'floatingActionLabel'
    | 'showFloatingActionButton'
    | 'activePrivateChannel'
    | 'mentionCandidates'
  >;
  handleCopyInternalLink: (link: string) => void;
  sharePreview: SharePreview;
  clipboardToastId: number;
};

function AccessPreviewItem({
  label,
  value,
  tooltip,
}: {
  label: string;
  value: string | null;
  tooltip: string;
}) {
  return (
    <TooltipProvider delayDuration={180}>
      <Tooltip>
        <TooltipTrigger asChild>
          <div>
            <dt>{label}</dt>
            <dd>{value ?? '-'}</dd>
          </div>
        </TooltipTrigger>
        <TooltipContent>{tooltip}</TooltipContent>
      </Tooltip>
    </TooltipProvider>
  );
}

export function DesktopShellOverlays({
  actions,
  dialogs,
  t,
  viewModels,
  handleCopyInternalLink,
  sharePreview,
  clipboardToastId,
}: DesktopShellOverlaysProps) {
  const {
    clearReply,
    clearRepost,
    handleAttachmentSelection,
    handleCreateGameRoom,
    handleCreateLiveSession,
    handleCreatePrivateChannel,
    handleJoinChannelAccess,
    handleProfileAvatarFile,
    handlePublish,
    handleRemoveDraftAttachment,
    handleShareChannelAccess,
    openFloatingActionDialog,
  } = actions;
  const {
    channelDialogOpen,
    channelSettingsDialogOpen,
    composeDialogOpen,
    gameCreateDialogOpen,
    leaveChannelDialogOpen,
    liveCreateDialogOpen,
    profileAvatarCropFile,
    profileAvatarCropOpen,
    setChannelDialogOpen,
    setChannelSettingsDialogOpen,
    setComposeDialogOpen,
    setGameCreateDialogOpen,
    setLeaveChannelDialogOpen,
    setLiveCreateDialogOpen,
    setProfileAvatarCropFile,
    setProfileAvatarCropOpen,
  } = dialogs;
  const {
    confirmImport: handleConfirmShareImport,
    data: sharePreviewData,
    error: sharePreviewError,
    handleOpenChange: handleSharePreviewOpenChange,
    importPending: shareImportPending,
    loading: sharePreviewLoading,
    open: sharePreviewOpen,
    token: sharePreviewToken,
  } = sharePreview;
  const {
    activeComposeAudienceLabel,
    activeChannelPanelState,
    channelAudienceOptions,
    composerDraftViews,
    composerSourcePreview,
    floatingActionLabel,
    showFloatingActionButton,
    activePrivateChannel,
    mentionCandidates,
  } = viewModels;
  const {
    activeTopic,
    attachmentInputKey,
    channelActionPending,
    channelAudienceInput,
    channelError,
    channelLabelInput,
    composer,
    composerError,
    gameCreatePending,
    gameDescription,
    gameError,
    gameParticipantsInput,
    gameTitle,
    inviteOutput,
    inviteOutputLabel,
    inviteTokenInput,
    knownAuthorsByPubkey,
    localProfile,
    liveCreatePending,
    liveDescription,
    liveError,
    liveTitle,
    replyTarget,
    repostTarget,
    syncStatus,
  } = useDesktopShellStore(
    useShallow((s) => ({
      activeTopic: s.activeTopic,
      attachmentInputKey: s.attachmentInputKey,
      channelActionPending: s.channelActionPending,
      channelAudienceInput: s.channelAudienceInput,
      channelError: s.channelError,
      channelLabelInput: s.channelLabelInput,
      composer: s.composer,
      composerError: s.composerError,
      gameCreatePending: s.gameCreatePending,
      gameDescription: s.gameDescription,
      gameError: s.gameError,
      gameParticipantsInput: s.gameParticipantsInput,
      gameTitle: s.gameTitle,
      inviteOutput: s.inviteOutput,
      inviteOutputLabel: s.inviteOutputLabel,
      inviteTokenInput: s.inviteTokenInput,
      knownAuthorsByPubkey: s.knownAuthorsByPubkey,
      localProfile: s.localProfile,
      liveCreatePending: s.liveCreatePending,
      liveDescription: s.liveDescription,
      liveError: s.liveError,
      liveTitle: s.liveTitle,
      replyTarget: s.replyTarget,
      repostTarget: s.repostTarget,
      syncStatus: s.syncStatus,
    }))
  );
  const setChannelLabelInput = useDesktopShellFieldSetter('channelLabelInput');
  const setChannelAudienceInput = useDesktopShellFieldSetter('channelAudienceInput');
  const setInviteTokenInput = useDesktopShellFieldSetter('inviteTokenInput');
  const setComposer = useDesktopShellFieldSetter('composer');
  const setLiveTitle = useDesktopShellFieldSetter('liveTitle');
  const setLiveDescription = useDesktopShellFieldSetter('liveDescription');
  const setGameTitle = useDesktopShellFieldSetter('gameTitle');
  const setGameDescription = useDesktopShellFieldSetter('gameDescription');
  const setGameParticipantsInput = useDesktopShellFieldSetter('gameParticipantsInput');
  const previewOwnerProfile =
    sharePreviewData?.owner_pubkey === syncStatus.local_author_pubkey
      ? localProfile
      : sharePreviewData
        ? knownAuthorsByPubkey[sharePreviewData.owner_pubkey] ?? null
        : null;
  const previewOwnerLabel = sharePreviewData
    ? authorDisplayLabel(
        sharePreviewData.owner_pubkey,
        previewOwnerProfile?.display_name,
        previewOwnerProfile?.name
      )
    : null;
  const previewAudienceLabel = sharePreviewData
    ? t(
        sharePreviewData.kind === 'invite'
          ? 'channels:audienceOptions.invite_only'
          : sharePreviewData.kind === 'grant'
            ? 'channels:audienceOptions.friend_only'
            : 'channels:audienceOptions.friend_plus'
      )
    : null;

  return (
    <>
      <ImageCropDialog
        open={profileAvatarCropOpen}
        file={profileAvatarCropFile}
        title={t('profile:editor.picture')}
        description={t('profile:editor.pictureCropDescription', {
          defaultValue: 'Drag and zoom to choose the visible square for your avatar.',
        })}
        confirmLabel={t('common:actions.save')}
        onOpenChange={(open) => {
          setProfileAvatarCropOpen(open);
          if (!open) {
            setProfileAvatarCropFile(null);
          }
        }}
        onConfirm={async ({ croppedFile }) => {
          await handleProfileAvatarFile(croppedFile);
          setProfileAvatarCropOpen(false);
          setProfileAvatarCropFile(null);
        }}
      />

      <Dialog open={channelDialogOpen} onOpenChange={setChannelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:createDialogTitle')}</DialogTitle>
            <DialogDescription>{topicDisplayName(activeTopic)}</DialogDescription>
          </DialogHeader>
          <DialogBody>
            <PrivateChannelPanel
              status={activeChannelPanelState.status}
              error={channelError ?? activeChannelPanelState.error}
              pendingAction={channelActionPending}
              channelLabel={channelLabelInput}
              channelAudience={channelAudienceInput}
              channelAudienceOptions={channelAudienceOptions}
              inviteTokenInput={inviteTokenInput}
              inviteOutput={inviteOutput}
              inviteOutputLabel={inviteOutputLabel}
              onChannelLabelChange={setChannelLabelInput}
              onChannelAudienceChange={setChannelAudienceInput}
              onInviteTokenChange={setInviteTokenInput}
              onCreateChannel={(event) => void handleCreatePrivateChannel(event)}
              onJoin={(event) => void handleJoinChannelAccess(event)}
              onCopyInviteOutput={handleCopyInternalLink}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={channelSettingsDialogOpen} onOpenChange={setChannelSettingsDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:settings.title')}</DialogTitle>
          </DialogHeader>
          <DialogBody>
            {activePrivateChannel ? (
              <PrivateChannelSettingsPanel
                error={channelError ?? activeChannelPanelState.error}
                pendingAction={channelActionPending}
                channel={activePrivateChannel}
                inviteOutput={inviteOutput}
                inviteOutputLabel={inviteOutputLabel}
                onShare={() => void handleShareChannelAccess()}
                onCopyInviteOutput={handleCopyInternalLink}
              />
            ) : (
              <Notice>{t('channels:selectChannelNotice')}</Notice>
            )}
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={leaveChannelDialogOpen} onOpenChange={setLeaveChannelDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:leaveDialog.title')}</DialogTitle>
            <DialogDescription>{t('channels:leaveDialog.description')}</DialogDescription>
          </DialogHeader>
          <DialogBody>
            <div className='ui-dialog-footer'>
              <Button
                variant='secondary'
                type='button'
                onClick={() => setLeaveChannelDialogOpen(false)}
              >
                {t('channels:leaveDialog.no')}
              </Button>
              <Button
                type='button'
                disabled={channelActionPending === 'leave'}
                onClick={() =>
                  void dialogs.confirmLeaveChannel(actions.handleLeavePrivateChannel)
                }
              >
                {t('channels:leaveDialog.yes')}
              </Button>
            </div>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog
        open={sharePreviewOpen}
        onOpenChange={handleSharePreviewOpenChange}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:previewDialog.title')}</DialogTitle>
          </DialogHeader>
          <DialogBody>
            {sharePreviewLoading ? <Notice>{t('channels:loading')}</Notice> : null}
            {sharePreviewError ? <Notice tone='destructive'>{sharePreviewError}</Notice> : null}
            {sharePreviewData ? (
              <dl className='access-preview-list'>
                <AccessPreviewItem
                  label={t('common:labels.owner')}
                  value={previewOwnerLabel}
                  tooltip={sharePreviewData.owner_pubkey}
                />
                <AccessPreviewItem
                  label={t('common:labels.sourceTopic')}
                  value={sharePreviewData.topic_id}
                  tooltip={sharePreviewData.topic_id}
                />
                <AccessPreviewItem
                  label={t('channels:previewDialog.channel')}
                  value={sharePreviewData.channel_label}
                  tooltip={sharePreviewData.channel_id}
                />
                <AccessPreviewItem
                  label={t('common:labels.audience')}
                  value={previewAudienceLabel}
                  tooltip={`${sharePreviewData.kind} / ${sharePreviewData.epoch_id}`}
                />
              </dl>
            ) : null}
            <div className='ui-dialog-footer'>
              {sharePreviewToken ? (
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() => handleCopyInternalLink(sharePreviewToken)}
                >
                  {t('channels:previewDialog.copyToken')}
                </Button>
              ) : null}
              <Button
                variant='secondary'
                type='button'
                onClick={() => handleSharePreviewOpenChange(false)}
              >
                {t('common:actions.cancel')}
              </Button>
              <Button
                type='button'
                disabled={sharePreviewLoading || shareImportPending || !sharePreviewData}
                onClick={() => void handleConfirmShareImport()}
              >
                {shareImportPending ? t('common:actions.join') : t('channels:previewDialog.import')}
              </Button>
            </div>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={composeDialogOpen} onOpenChange={setComposeDialogOpen}>
        <DialogContent className='shell-compose-dialog'>
          <DialogHeader>
            <DialogTitle>
              {replyTarget
                ? t('common:actions.reply')
                : repostTarget
                  ? t('common:actions.quoteRepost')
                  : t('common:actions.publish')}
            </DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <ComposerPanel
              value={composer}
              onChange={(event) => setComposer(event.target.value)}
              onValueChange={setComposer}
              mentionCandidates={mentionCandidates}
              onSubmit={(event) => void handlePublish(event)}
              attachmentInputKey={attachmentInputKey}
              onAttachmentSelection={(event) => {
                void handleAttachmentSelection(event);
              }}
              draftMediaItems={composerDraftViews}
              onRemoveDraftAttachment={handleRemoveDraftAttachment}
              composerError={composerError}
              audienceLabel={activeComposeAudienceLabel}
              sourcePreview={composerSourcePreview}
              replyTarget={
                replyTarget
                  ? {
                      content: replyTarget.content,
                      audienceLabel: replyTarget.audience_label,
                    }
                  : null
              }
              repostTarget={
                repostTarget
                  ? {
                      content: repostTarget.content,
                      authorLabel: authorDisplayLabel(
                        repostTarget.author_pubkey,
                        repostTarget.author_display_name,
                        repostTarget.author_name
                      ),
                    }
                  : null
              }
              onClearReply={clearReply}
              onClearRepost={clearRepost}
              attachmentsDisabled={Boolean(repostTarget)}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={liveCreateDialogOpen} onOpenChange={setLiveCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('live:actions.start')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={(event) => void handleCreateLiveSession(event)}
              aria-busy={liveCreatePending}
            >
              <Label>
                <span>{t('live:fields.title')}</span>
                <Input
                  value={liveTitle}
                  onChange={(event) => setLiveTitle(event.target.value)}
                  placeholder={t('live:fields.placeholders.title')}
                  disabled={liveCreatePending}
                />
              </Label>
              <Label>
                <span>{t('live:fields.description')}</span>
                <Textarea
                  value={liveDescription}
                  onChange={(event) => setLiveDescription(event.target.value)}
                  placeholder={t('live:fields.placeholders.description')}
                  disabled={liveCreatePending}
                />
              </Label>
              {liveError ? <p className='error error-inline'>{liveError}</p> : null}
              <Button type='submit' disabled={liveCreatePending}>
                {t('live:actions.start')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog open={gameCreateDialogOpen} onOpenChange={setGameCreateDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('game:actions.createRoom')}</DialogTitle>
            <DialogDescription>
              {t('common:labels.audience')}: {activeComposeAudienceLabel}
            </DialogDescription>
          </DialogHeader>
          <DialogBody>
            <form
              className='composer composer-compact'
              onSubmit={(event) => void handleCreateGameRoom(event)}
              aria-busy={gameCreatePending}
            >
              <Label>
                <span>{t('game:fields.title')}</span>
                <Input
                  value={gameTitle}
                  onChange={(event) => setGameTitle(event.target.value)}
                  placeholder={t('game:fields.placeholders.title')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.description')}</span>
                <Textarea
                  value={gameDescription}
                  onChange={(event) => setGameDescription(event.target.value)}
                  placeholder={t('game:fields.placeholders.description')}
                  disabled={gameCreatePending}
                />
              </Label>
              <Label>
                <span>{t('game:fields.participants')}</span>
                <Input
                  value={gameParticipantsInput}
                  onChange={(event) => setGameParticipantsInput(event.target.value)}
                  placeholder={t('game:fields.placeholders.participants')}
                  disabled={gameCreatePending}
                />
              </Label>
              {gameError ? <p className='error error-inline'>{gameError}</p> : null}
              <Button type='submit' disabled={gameCreatePending}>
                {t('game:actions.createRoom')}
              </Button>
            </form>
          </DialogBody>
        </DialogContent>
      </Dialog>

      {showFloatingActionButton ? (
        <Button
          className='shell-fab'
          variant='primary'
          size='icon'
          type='button'
          data-testid='shell-fab'
          aria-label={floatingActionLabel}
          onClick={openFloatingActionDialog}
        >
          <Plus className='size-5' aria-hidden='true' />
        </Button>
      ) : null}

      {clipboardToastId > 0 ? (
        <div className='pointer-events-none fixed right-4 bottom-4 z-[90] w-[calc(100vw-2rem)] max-w-xs'>
          <Notice
            key={clipboardToastId}
            role='status'
            aria-live='polite'
            aria-atomic='true'
            tone='accent'
            className='pointer-events-auto'
          >
            {t('common:feedback.copiedToClipboard')}
          </Notice>
        </div>
      ) : null}
    </>
  );
}
