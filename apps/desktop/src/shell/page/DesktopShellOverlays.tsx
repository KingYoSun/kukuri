import type { ChangeEvent, Dispatch, FormEvent, SetStateAction } from 'react';

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

import type { ChannelAccessTokenPreview } from '@/lib/api';
import { authorDisplayLabel } from '@/shell/selectors';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import type { Translate } from '@/shell/actions/shared';
import { useDesktopShellViewModels } from '@/shell/useDesktopShellViewModels';

type ViewModels = ReturnType<typeof useDesktopShellViewModels>;

type DesktopShellOverlaysProps = {
  t: Translate;
  activeTopic: string;
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
  >;
  profileAvatarCropOpen: boolean;
  profileAvatarCropFile: File | null;
  setProfileAvatarCropOpen: Dispatch<SetStateAction<boolean>>;
  setProfileAvatarCropFile: Dispatch<SetStateAction<File | null>>;
  handleProfileAvatarFile: (file: File) => Promise<void>;
  channelDialogOpen: boolean;
  setChannelDialogOpen: Dispatch<SetStateAction<boolean>>;
  channelSettingsDialogOpen: boolean;
  setChannelSettingsDialogOpen: Dispatch<SetStateAction<boolean>>;
  leaveChannelDialogOpen: boolean;
  setLeaveChannelDialogOpen: (open: boolean) => void;
  handleConfirmLeaveChannel: () => Promise<void>;
  sharePreviewOpen: boolean;
  setSharePreviewOpen: Dispatch<SetStateAction<boolean>>;
  sharePreviewToken: string | null;
  setSharePreviewToken: Dispatch<SetStateAction<string | null>>;
  sharePreviewData: ChannelAccessTokenPreview | null;
  setSharePreviewData: Dispatch<SetStateAction<ChannelAccessTokenPreview | null>>;
  sharePreviewLoading: boolean;
  sharePreviewError: string | null;
  setSharePreviewError: Dispatch<SetStateAction<string | null>>;
  shareImportPending: boolean;
  handleConfirmShareImport: () => Promise<void>;
  handleCreatePrivateChannel: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  handleJoinChannelAccess: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  handleShareChannelAccess: () => Promise<void>;
  handleCopyInternalLink: (link: string) => void;
  composeDialogOpen: boolean;
  setComposeDialogOpen: Dispatch<SetStateAction<boolean>>;
  handlePublish: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  handleAttachmentSelection: (event: ChangeEvent<HTMLInputElement>) => Promise<void>;
  handleRemoveDraftAttachment: (itemId: string) => void;
  clearReply: () => void;
  clearRepost: () => void;
  liveCreateDialogOpen: boolean;
  setLiveCreateDialogOpen: Dispatch<SetStateAction<boolean>>;
  handleCreateLiveSession: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  gameCreateDialogOpen: boolean;
  setGameCreateDialogOpen: Dispatch<SetStateAction<boolean>>;
  handleCreateGameRoom: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  openFloatingActionDialog: () => void;
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
  t,
  activeTopic,
  viewModels,
  profileAvatarCropOpen,
  profileAvatarCropFile,
  setProfileAvatarCropOpen,
  setProfileAvatarCropFile,
  handleProfileAvatarFile,
  channelDialogOpen,
  setChannelDialogOpen,
  channelSettingsDialogOpen,
  setChannelSettingsDialogOpen,
  leaveChannelDialogOpen,
  setLeaveChannelDialogOpen,
  handleConfirmLeaveChannel,
  sharePreviewOpen,
  setSharePreviewOpen,
  sharePreviewToken,
  setSharePreviewToken,
  sharePreviewData,
  setSharePreviewData,
  sharePreviewLoading,
  sharePreviewError,
  setSharePreviewError,
  shareImportPending,
  handleConfirmShareImport,
  handleCreatePrivateChannel,
  handleJoinChannelAccess,
  handleShareChannelAccess,
  handleCopyInternalLink,
  composeDialogOpen,
  setComposeDialogOpen,
  handlePublish,
  handleAttachmentSelection,
  handleRemoveDraftAttachment,
  clearReply,
  clearRepost,
  liveCreateDialogOpen,
  setLiveCreateDialogOpen,
  handleCreateLiveSession,
  gameCreateDialogOpen,
  setGameCreateDialogOpen,
  handleCreateGameRoom,
  openFloatingActionDialog,
  clipboardToastId,
}: DesktopShellOverlaysProps) {
  const {
    activeComposeAudienceLabel,
    activeChannelPanelState,
    channelAudienceOptions,
    composerDraftViews,
    composerSourcePreview,
    floatingActionLabel,
    showFloatingActionButton,
    activePrivateChannel,
  } = viewModels;
  const {
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
  } = useDesktopShellStore();
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
            <DialogDescription>{activeTopic}</DialogDescription>
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
                onClick={() => void handleConfirmLeaveChannel()}
              >
                {t('channels:leaveDialog.yes')}
              </Button>
            </div>
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog
        open={sharePreviewOpen}
        onOpenChange={(open) => {
          setSharePreviewOpen(open);
          if (!open) {
            setSharePreviewError(null);
            setSharePreviewData(null);
            setSharePreviewToken(null);
          }
        }}
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
                onClick={() => setSharePreviewOpen(false)}
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
