import type { ChangeEventHandler, FormEventHandler } from 'react';

import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';

import { ComposerDraftPreviewList } from './ComposerDraftPreviewList';
import { type ComposerDraftMediaView } from './types';

type ReplyTargetView = {
  content: string;
  audienceLabel: string;
};

type RepostTargetView = {
  content: string;
  authorLabel: string;
};

type ComposerPanelProps = {
  value: string;
  onChange: ChangeEventHandler<HTMLTextAreaElement>;
  onSubmit: FormEventHandler<HTMLFormElement>;
  attachmentInputKey: number;
  onAttachmentSelection: ChangeEventHandler<HTMLInputElement>;
  draftMediaItems: ComposerDraftMediaView[];
  onRemoveDraftAttachment: (itemId: string) => void;
  composerError?: string | null;
  audienceLabel: string;
  replyTarget?: ReplyTargetView | null;
  repostTarget?: RepostTargetView | null;
  onClearReply: () => void;
  onClearRepost?: () => void;
  attachmentsDisabled?: boolean;
};

export function ComposerPanel({
  value,
  onChange,
  onSubmit,
  attachmentInputKey,
  onAttachmentSelection,
  draftMediaItems,
  onRemoveDraftAttachment,
  composerError,
  audienceLabel,
  replyTarget,
  repostTarget,
  onClearReply,
  onClearRepost,
  attachmentsDisabled = false,
}: ComposerPanelProps) {
  const { t } = useTranslation(['common']);
  const clearActiveTarget = replyTarget ? onClearReply : onClearRepost;
  const bannerAriaLabel = replyTarget ? t('composer.clearReply') : t('composer.clearQuoteRepost');

  return (
    <form className='composer' onSubmit={onSubmit}>
      {replyTarget || repostTarget ? (
        <div className='reply-banner'>
          <strong>{replyTarget ? t('composer.replying') : t('composer.quoteReposting')}</strong>
          <Button
            className='shell-icon-button'
            variant='ghost'
            size='icon'
            type='button'
            aria-label={bannerAriaLabel}
            onClick={() => clearActiveTarget?.()}
          >
            <X className='size-5' aria-hidden='true' />
          </Button>
        </div>
      ) : null}

      <Textarea
        value={value}
        onChange={onChange}
        placeholder={
          replyTarget
            ? t('composer.writeReply')
            : repostTarget
              ? t('composer.writeQuoteRepost')
              : t('composer.writePost')
        }
      />

      <Label className='file-field file-field-compact'>
        <span>{t('common:fallbacks.attachment')}</span>
        <Input
          key={attachmentInputKey}
          aria-label={t('common:fallbacks.attachment')}
          type='file'
          accept='image/*,video/*'
          multiple
          disabled={attachmentsDisabled}
          onChange={onAttachmentSelection}
        />
      </Label>

      {composerError ? <p className='error error-inline'>{composerError}</p> : null}

      <ComposerDraftPreviewList items={draftMediaItems} onRemove={onRemoveDraftAttachment} />

      <div className='topic-diagnostic topic-diagnostic-secondary'>
        <span>{t('labels.audience')}: {audienceLabel}</span>
      </div>

      <Button type='submit'>
        {replyTarget
          ? t('actions.reply')
          : repostTarget
            ? t('actions.quoteRepost')
            : t('actions.publish')}
      </Button>
    </form>
  );
}
