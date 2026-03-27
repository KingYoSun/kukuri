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
  onClearReply: () => void;
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
  onClearReply,
}: ComposerPanelProps) {
  const { t } = useTranslation(['common']);

  return (
    <form className='composer' onSubmit={onSubmit}>
      {replyTarget ? (
        <div className='reply-banner'>
          <strong>{t('composer.replying')}</strong>
          <Button
            className='shell-icon-button'
            variant='ghost'
            size='icon'
            type='button'
            aria-label={t('composer.clearReply')}
            onClick={onClearReply}
          >
            <X className='size-5' aria-hidden='true' />
          </Button>
        </div>
      ) : null}

      <Textarea
        value={value}
        onChange={onChange}
        placeholder={replyTarget ? t('composer.writeReply') : t('composer.writePost')}
      />

      <Label className='file-field file-field-compact'>
        <span>{t('common:fallbacks.attachment')}</span>
        <Input
          key={attachmentInputKey}
          aria-label={t('common:fallbacks.attachment')}
          type='file'
          accept='image/*,video/*'
          multiple
          onChange={onAttachmentSelection}
        />
      </Label>

      {composerError ? <p className='error error-inline'>{composerError}</p> : null}

      <ComposerDraftPreviewList items={draftMediaItems} onRemove={onRemoveDraftAttachment} />

      <div className='topic-diagnostic topic-diagnostic-secondary'>
        <span>{t('labels.audience')}: {audienceLabel}</span>
      </div>

      <Button type='submit'>{replyTarget ? t('actions.reply') : t('actions.publish')}</Button>
    </form>
  );
}
