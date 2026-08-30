import type { ChangeEventHandler, FormEventHandler, KeyboardEventHandler } from 'react';

import { X } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Textarea } from '@/components/ui/textarea';

import { AuthorAvatar } from './AuthorAvatar';
import { ComposerDraftPreviewList } from './ComposerDraftPreviewList';
import { MentionHoverCard } from './MentionHoverCard';
import { PostCard } from './PostCard';
import { useMentionAutocomplete } from './useMentionAutocomplete';
import {
  type ComposerDraftMediaView,
  type MentionAuthorView,
  type MentionCandidate,
  type PostCardView,
} from './types';

function mentionAuthorFromCandidate(candidate: MentionCandidate): MentionAuthorView {
  return {
    pubkey: candidate.pubkey,
    label: candidate.label,
    displayName: candidate.displayName,
    name: candidate.name,
    aboutPreview: candidate.about?.slice(0, 50) ?? null,
    picture: candidate.picture,
  };
}

type ReplyTargetView = {
  content: string;
  audienceLabel: string;
};

type RepostTargetView = {
  content: string;
  authorLabel: string;
};

type ComposerPanelProps = {
  mode?: 'post' | 'reply' | 'message';
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
  sourcePreview?: PostCardView | null;
  onClearReply: () => void;
  onClearRepost?: () => void;
  attachmentsDisabled?: boolean;
  submitDisabled?: boolean;
  mentionCandidates?: MentionCandidate[];
  onValueChange?: (next: string) => void;
};

const EMPTY_MENTION_CANDIDATES: MentionCandidate[] = [];

export function ComposerPanel({
  mode = 'post',
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
  sourcePreview,
  onClearReply,
  onClearRepost,
  attachmentsDisabled = false,
  submitDisabled = false,
  mentionCandidates = EMPTY_MENTION_CANDIDATES,
  onValueChange,
}: ComposerPanelProps) {
  const { t } = useTranslation(['common']);
  const clearActiveTarget = replyTarget ? onClearReply : onClearRepost;
  const bannerAriaLabel = replyTarget ? t('composer.clearReply') : t('composer.clearQuoteRepost');
  const {
    textareaRef: mentionTextareaRef,
    isOpen: mentionOpen,
    items: mentionItems,
    activeIndex: mentionActiveIndex,
    onKeyDown: onMentionKeyDown,
    onSelectionChange: onMentionSelectionChange,
    selectCandidate: selectMention,
    setActiveIndex: setMentionActiveIndex,
  } = useMentionAutocomplete({
    value,
    candidates: mentionCandidates,
    onValueChange,
  });
  const onComposerKeyDown: KeyboardEventHandler<HTMLTextAreaElement> = (event) => {
    onMentionKeyDown(event);
    if (event.defaultPrevented || submitDisabled || event.key !== 'Enter' || !event.ctrlKey) {
      return;
    }
    event.preventDefault();
    event.currentTarget.form?.requestSubmit();
  };

  return (
    <form className='composer' onSubmit={onSubmit}>
      {replyTarget || repostTarget ? (
        <div className='reply-banner'>
          <span className='composer-target-summary'>
            <strong>{replyTarget ? t('composer.replying') : t('composer.quoteReposting')}</strong>
            {replyTarget ? (
              <span className='post-copy-wrap'>{replyTarget.content}</span>
            ) : repostTarget ? (
              <span className='post-copy-wrap'>
                {t('composer.sourcePost')} · {repostTarget.authorLabel}: {repostTarget.content}
              </span>
            ) : null}
          </span>
          <IconButton
            className='shell-icon-button'
            variant='ghost'
            type='button'
            label={bannerAriaLabel}
            onClick={() => clearActiveTarget?.()}
          >
            <X className='size-5' aria-hidden='true' />
          </IconButton>
        </div>
      ) : null}

      {sourcePreview ? (
        <div className='composer-source-preview'>
          <div className='topic-diagnostic topic-diagnostic-secondary'>
            <span>{t('composer.sourcePost')}</span>
            <span>{sourcePreview.audienceChipLabel ?? sourcePreview.post.audience_label}</span>
          </div>
          <PostCard
            view={sourcePreview}
            onOpenAuthor={() => undefined}
            onOpenThread={() => undefined}
            onReply={() => undefined}
            readOnly
          />
        </div>
      ) : null}

      <div className='composer-mention-anchor'>
        <Textarea
          ref={mentionTextareaRef}
          value={value}
          onChange={(event) => {
            onChange(event);
            onMentionSelectionChange();
          }}
          onKeyDown={onComposerKeyDown}
          onKeyUp={onMentionSelectionChange}
          onClick={onMentionSelectionChange}
          onSelect={onMentionSelectionChange}
          aria-expanded={mentionOpen}
          aria-controls={mentionOpen ? 'composer-mention-listbox' : undefined}
          placeholder={
            replyTarget || mode === 'reply'
              ? t('composer.writeReply')
              : repostTarget
                ? t('composer.writeQuoteRepost')
                : mode === 'message'
                  ? t('composer.writeMessage')
                  : t('composer.writePost')
          }
        />
        {mentionOpen ? (
          <ul
            id='composer-mention-listbox'
            className='composer-mention-list'
            role='listbox'
            aria-label={t('composer.mentionSuggestionsLabel')}
          >
            {mentionItems.map((candidate, index) => (
              <li key={candidate.pubkey} role='presentation'>
                <MentionHoverCard
                  pubkey={candidate.pubkey}
                  label={candidate.label}
                  author={mentionAuthorFromCandidate(candidate)}
                >
                  <button
                    type='button'
                    role='option'
                    aria-selected={index === mentionActiveIndex}
                    className={
                      index === mentionActiveIndex
                        ? 'composer-mention-option composer-mention-option-active'
                        : 'composer-mention-option'
                    }
                    onMouseDown={(event) => {
                      event.preventDefault();
                      selectMention(candidate);
                    }}
                    onMouseEnter={() => setMentionActiveIndex(index)}
                  >
                    <AuthorAvatar label={candidate.label} picture={candidate.picture ?? null} size='sm' />
                    <span className='composer-mention-option-text'>
                      <span className='composer-mention-option-label'>{candidate.label}</span>
                    </span>
                  </button>
                </MentionHoverCard>
              </li>
            ))}
          </ul>
        ) : null}
      </div>

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

      <Button type='submit' disabled={submitDisabled}>
        {replyTarget || mode === 'reply'
          ? t('actions.reply')
          : repostTarget
            ? t('actions.quoteRepost')
            : mode === 'message'
              ? t('actions.send')
              : t('actions.publish')}
      </Button>
    </form>
  );
}
