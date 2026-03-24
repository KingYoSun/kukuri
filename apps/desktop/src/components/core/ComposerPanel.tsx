import type { ChangeEventHandler, FormEventHandler } from 'react';

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
  return (
    <form className='composer' onSubmit={onSubmit}>
      {replyTarget ? (
        <div className='reply-banner'>
          <div>
            <strong>Replying</strong>
            <p>{replyTarget.content}</p>
            <small>Audience: {replyTarget.audienceLabel}</small>
          </div>
          <Button variant='secondary' type='button' onClick={onClearReply}>
            Clear
          </Button>
        </div>
      ) : null}

      <Textarea
        value={value}
        onChange={onChange}
        placeholder={replyTarget ? 'Write a reply' : 'Write a post'}
      />

      <Label className='file-field'>
        <span>Attach</span>
        <Input
          key={attachmentInputKey}
          aria-label='Attach'
          type='file'
          accept='image/*,video/*'
          multiple
          onChange={onAttachmentSelection}
        />
      </Label>

      {composerError ? <p className='error error-inline'>{composerError}</p> : null}

      <ComposerDraftPreviewList items={draftMediaItems} onRemove={onRemoveDraftAttachment} />

      <div className='topic-diagnostic topic-diagnostic-secondary'>
        <span>Audience: {audienceLabel}</span>
      </div>

      <Button type='submit'>{replyTarget ? 'Reply' : 'Publish'}</Button>
    </form>
  );
}
