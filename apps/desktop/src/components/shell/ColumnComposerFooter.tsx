import { MessageSquare, PenLine, Reply } from 'lucide-react';
import type { ChangeEvent, FormEvent } from 'react';
import { useTranslation } from 'react-i18next';

import { ComposerPanel } from '@/components/core/ComposerPanel';
import type { ComposerDraftMediaView, MentionCandidate } from '@/components/core/types';
import { Button } from '@/components/ui/button';
import { formatLocalizedBytes } from '@/i18n/format';
import { authorDisplayLabel } from '@/shell/presentation';
import {
  columnDraftKey,
  createColumnDraft,
  setColumnDraft,
  type ColumnDraftTarget,
} from '@/shell/slices/columnDrafts';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';

type ColumnComposerFooterProps = {
  active: boolean;
  destinationLabel: string;
  locale: string;
  mentionCandidates?: MentionCandidate[];
  onActivate: () => void;
  onAttachmentSelection: (
    target: ColumnDraftTarget,
    event: ChangeEvent<HTMLInputElement>
  ) => Promise<void>;
  onRemoveAttachment: (target: ColumnDraftTarget, itemId: string) => void;
  onSubmit: (target: ColumnDraftTarget, event: FormEvent<HTMLFormElement>) => Promise<void>;
  target: ColumnDraftTarget;
};

const ICON_BY_ACTION = {
  post: PenLine,
  reply: Reply,
  message: MessageSquare,
} as const;

const LABEL_KEY_BY_ACTION = {
  post: 'actions.publish',
  reply: 'actions.reply',
  message: 'actions.message',
} as const;

export function ColumnComposerFooter({
  active,
  destinationLabel,
  locale,
  mentionCandidates,
  onActivate,
  onAttachmentSelection,
  onRemoveAttachment,
  onSubmit,
  target,
}: ColumnComposerFooterProps) {
  const { t } = useTranslation(['common']);
  const key = columnDraftKey(target);
  const storedDraft = useDesktopShellStore((state) => state.columnDraftsByKey[key]);
  const draft = storedDraft ?? createColumnDraft(target);
  const setColumnDraftsByKey = useDesktopShellFieldSetter('columnDraftsByKey');
  const Icon = ICON_BY_ACTION[target.action];
  const actionLabel = t(LABEL_KEY_BY_ACTION[target.action]);
  const draftMediaViews: ComposerDraftMediaView[] = draft.mediaItems.map((item) => ({
    id: item.id,
    sourceName: item.source_name,
    previewUrl: item.preview_url,
    attachments: item.attachments.map((attachment, index) => ({
      key: `${item.id}:${index}`,
      label: attachment.role ?? attachment.mime,
      mime: attachment.mime,
      byteSizeLabel: formatLocalizedBytes(attachment.byte_size, locale),
    })),
  }));
  const updateDraft = (update: Parameters<typeof setColumnDraft>[2]) => {
    setColumnDraftsByKey((current) => setColumnDraft(current, target, update));
  };

  if (!draft.expanded) {
    return (
      <Button
        className='shell-column-primary-action min-h-11 min-w-11'
        variant='primary'
        size={active ? 'default' : 'icon'}
        type='button'
        aria-label={`${actionLabel} to ${destinationLabel}`}
        onClick={() => {
          onActivate();
          updateDraft((current) => ({ ...current, expanded: true, error: null }));
        }}
      >
        <Icon className='size-4' aria-hidden='true' />
        {active ? <span>{actionLabel}</span> : null}
      </Button>
    );
  }

  return (
    <div className='shell-column-composer'>
      <div className='shell-column-composer-heading'>
        <strong>{actionLabel}</strong>
        <Button
          variant='ghost'
          size='sm'
          className='min-h-11'
          type='button'
          onClick={() => updateDraft((current) => ({ ...current, expanded: false }))}
        >
          {t('actions.close')}
        </Button>
      </div>
      <ComposerPanel
        mode={target.action}
        value={draft.content}
        onChange={(event) =>
          updateDraft((current) => ({ ...current, content: event.target.value, error: null }))
        }
        onValueChange={(content) =>
          updateDraft((current) => ({ ...current, content, error: null }))
        }
        onSubmit={(event) => void onSubmit(target, event)}
        attachmentInputKey={draft.attachmentInputKey}
        onAttachmentSelection={(event) => void onAttachmentSelection(target, event)}
        draftMediaItems={draftMediaViews}
        onRemoveDraftAttachment={(itemId) => onRemoveAttachment(target, itemId)}
        composerError={draft.error}
        audienceLabel={destinationLabel}
        replyTarget={
          draft.replyTarget
            ? {
                content: draft.replyTarget.content,
                audienceLabel: draft.replyTarget.audience_label,
              }
            : null
        }
        repostTarget={
          draft.repostTarget
            ? {
                content: draft.repostTarget.content,
                authorLabel: authorDisplayLabel(
                  draft.repostTarget.author_pubkey,
                  draft.repostTarget.author_display_name,
                  draft.repostTarget.author_name
                ),
              }
            : null
        }
        onClearReply={() =>
          updateDraft((current) => ({ ...current, replyTarget: null, error: null }))
        }
        onClearRepost={() =>
          updateDraft((current) => ({ ...current, repostTarget: null, error: null }))
        }
        attachmentsDisabled={draft.pending || Boolean(draft.repostTarget)}
        mentionCandidates={mentionCandidates}
      />
    </div>
  );
}
