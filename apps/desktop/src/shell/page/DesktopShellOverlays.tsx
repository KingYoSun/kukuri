import type { ChangeEvent, Dispatch, FormEvent, SetStateAction } from 'react';

import { Plus } from 'lucide-react';

import { ComposerPanel } from '@/components/core/ComposerPanel';
import { PrivateChannelPanel } from '@/components/extended/PrivateChannelPanel';
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

import type { ChannelAccessTokenPreview } from '@/lib/api';
import type { InternalSmartReference } from '@/lib/internalLinks';
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
    | 'privateChannelListItems'
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
  sharePreviewOpen: boolean;
  setSharePreviewOpen: Dispatch<SetStateAction<boolean>>;
  sharePreviewToken: string | null;
  setSharePreviewToken: Dispatch<SetStateAction<string | null>>;
  sharePreviewData: ChannelAccessTokenPreview | null;
  setSharePreviewData: Dispatch<SetStateAction<ChannelAccessTokenPreview | null>>;
  sharePreviewLoading: boolean;
  sharePreviewError: string | null;
  setSharePreviewError: Dispatch<SetStateAction<string | null>>;
  sharePreviewShowRaw: boolean;
  setSharePreviewShowRaw: Dispatch<SetStateAction<boolean>>;
  shareImportPending: boolean;
  handleConfirmShareImport: () => Promise<void>;
  handleCreatePrivateChannel: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  handleJoinChannelAccess: (event: FormEvent<HTMLFormElement>) => Promise<void>;
  handleSelectPrivateChannel: (topicId: string, channelId: string) => void;
  handleShareChannelAccess: () => Promise<void>;
  handleActivateReference: (reference: InternalSmartReference) => Promise<void>;
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
  sharePreviewOpen,
  setSharePreviewOpen,
  sharePreviewToken,
  setSharePreviewToken,
  sharePreviewData,
  setSharePreviewData,
  sharePreviewLoading,
  sharePreviewError,
  setSharePreviewError,
  sharePreviewShowRaw,
  setSharePreviewShowRaw,
  shareImportPending,
  handleConfirmShareImport,
  handleCreatePrivateChannel,
  handleJoinChannelAccess,
  handleSelectPrivateChannel,
  handleShareChannelAccess,
  handleActivateReference,
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
    privateChannelListItems,
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
    liveCreatePending,
    liveDescription,
    liveError,
    liveTitle,
    replyTarget,
    repostTarget,
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
            <DialogTitle>{t('channels:title')}</DialogTitle>
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
              channels={privateChannelListItems}
              selectedChannel={activePrivateChannel}
              onChannelLabelChange={setChannelLabelInput}
              onChannelAudienceChange={setChannelAudienceInput}
              onInviteTokenChange={setInviteTokenInput}
              onCreateChannel={(event) => void handleCreatePrivateChannel(event)}
              onJoin={(event) => void handleJoinChannelAccess(event)}
              onSelectChannel={(channelId) => handleSelectPrivateChannel(activeTopic, channelId)}
              onShare={() => void handleShareChannelAccess()}
              onActivateReference={(reference) => void handleActivateReference(reference)}
              onCopyInviteOutput={handleCopyInternalLink}
            />
          </DialogBody>
        </DialogContent>
      </Dialog>

      <Dialog
        open={sharePreviewOpen}
        onOpenChange={(open) => {
          setSharePreviewOpen(open);
          if (!open) {
            setSharePreviewShowRaw(false);
            setSharePreviewError(null);
            setSharePreviewData(null);
            setSharePreviewToken(null);
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('channels:previewDialog.title')}</DialogTitle>
            <DialogDescription>{t('channels:previewDialog.description')}</DialogDescription>
          </DialogHeader>
          <DialogBody>
            {sharePreviewLoading ? <Notice>{t('channels:loading')}</Notice> : null}
            {sharePreviewError ? <Notice tone='destructive'>{sharePreviewError}</Notice> : null}
            {sharePreviewData ? (
              <>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>
                    {t('common:labels.policy')}:{' '}
                    {t(`channels:previewDialog.tokenKinds.${sharePreviewData.kind}`)}
                  </span>
                  <span>
                    {t('common:labels.epoch')}: {sharePreviewData.epoch_id}
                  </span>
                </div>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>
                    {t('common:labels.owner')}: {sharePreviewData.owner_pubkey}
                  </span>
                  <span>
                    {t('common:labels.sourceTopic')}: {sharePreviewData.topic_id}
                  </span>
                </div>
                <div className='topic-diagnostic topic-diagnostic-secondary'>
                  <span>
                    {t('channels:previewDialog.channel')}: {sharePreviewData.channel_label}
                  </span>
                  <span>
                    {t('channels:previewDialog.channelId')}: {sharePreviewData.channel_id}
                  </span>
                </div>
                {sharePreviewData.inviter_pubkey ? (
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      {t('channels:previewDialog.inviter')}: {sharePreviewData.inviter_pubkey}
                    </span>
                  </div>
                ) : null}
                {sharePreviewData.sponsor_pubkey ? (
                  <div className='topic-diagnostic topic-diagnostic-secondary'>
                    <span>
                      {t('channels:previewDialog.sponsor')}: {sharePreviewData.sponsor_pubkey}
                    </span>
                  </div>
                ) : null}
              </>
            ) : null}
            {sharePreviewShowRaw && sharePreviewToken ? (
              <code className='extended-inline-code'>{sharePreviewToken}</code>
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
              {sharePreviewToken ? (
                <Button
                  variant='secondary'
                  type='button'
                  onClick={() => setSharePreviewShowRaw((current) => !current)}
                >
                  {sharePreviewShowRaw
                    ? t('channels:previewDialog.hideRaw')
                    : t('channels:previewDialog.showRaw')}
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
