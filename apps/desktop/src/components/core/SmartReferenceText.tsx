import { Fragment, type MouseEvent, type KeyboardEvent } from 'react';
import { useTranslation } from 'react-i18next';

import {
  parseSmartText,
  shortenReferenceId,
  type InternalSmartReference,
} from '@/lib/internalLinks';
import { cn } from '@/lib/utils';

type SmartReferenceTextProps = {
  text: string;
  className?: string;
  onActivateReference?: (reference: InternalSmartReference) => void;
};

function tokenKindLabel(
  tokenKind: 'invite' | 'grant' | 'share',
  t: ReturnType<typeof useTranslation>['t']
): string {
  if (tokenKind === 'invite') {
    return t('channels:previewDialog.tokenKinds.invite');
  }
  if (tokenKind === 'grant') {
    return t('channels:previewDialog.tokenKinds.grant');
  }
  return t('channels:previewDialog.tokenKinds.share');
}

function referenceLabel(
  reference: InternalSmartReference,
  t: ReturnType<typeof useTranslation>['t']
): string {
  if (reference.kind === 'topic') {
    return reference.topic;
  }
  if (reference.kind === 'post') {
    return `${t('common:labels.post')} ${shortenReferenceId(
      reference.focusObjectId ?? reference.threadId
    )}`;
  }
  if (reference.kind === 'live') {
    return `${t('shell:primarySections.live')} ${shortenReferenceId(reference.sessionId)}`;
  }
  if (reference.kind === 'game') {
    return `${t('shell:primarySections.game')} ${shortenReferenceId(reference.roomId)}`;
  }
  return `${tokenKindLabel(reference.tokenKind, t)} ${t('channels:previewDialog.tokenLabelSuffix')}`;
}

function handleReferenceAction(
  event: MouseEvent<HTMLButtonElement> | KeyboardEvent<HTMLButtonElement>,
  reference: InternalSmartReference,
  onActivateReference?: (reference: InternalSmartReference) => void
) {
  event.preventDefault();
  event.stopPropagation();
  onActivateReference?.(reference);
}

export function SmartReferenceText({
  text,
  className,
  onActivateReference,
}: SmartReferenceTextProps) {
  const { t } = useTranslation(['channels', 'common', 'shell']);
  const lines = parseSmartText(text);

  return (
    <span className={cn('smart-reference-text', className)}>
      {lines.map((segments, lineIndex) => (
        <Fragment key={`${lineIndex}-${segments.length}`}>
          {segments.map((segment, segmentIndex) => {
            if (segment.kind === 'text') {
              return (
                <span key={`${lineIndex}-${segmentIndex}`} className={className}>
                  {segment.text}
                </span>
              );
            }
            const label = referenceLabel(segment.reference, t);
            return (
              <button
                key={`${lineIndex}-${segmentIndex}`}
                type='button'
                className='smart-reference-chip'
                title={
                  segment.reference.kind === 'share_token'
                    ? tokenKindLabel(segment.reference.tokenKind, t)
                    : segment.reference.route
                }
                onClick={(event) =>
                  handleReferenceAction(event, segment.reference, onActivateReference)
                }
                onKeyDown={(event) => {
                  if (event.key === 'Enter' || event.key === ' ') {
                    handleReferenceAction(event, segment.reference, onActivateReference);
                  }
                }}
              >
                {label}
              </button>
            );
          })}
          {lineIndex < lines.length - 1 ? <br /> : null}
        </Fragment>
      ))}
    </span>
  );
}
