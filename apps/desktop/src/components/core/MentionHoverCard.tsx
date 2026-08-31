import {
  cloneElement,
  useState,
  type KeyboardEvent as ReactKeyboardEvent,
  type MouseEvent as ReactMouseEvent,
  type ReactElement,
} from 'react';
import { useTranslation } from 'react-i18next';

import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import {
  ContextActionMenu,
  contextActionMenuPositionFromKeyboard,
  contextActionMenuPositionFromPointer,
  type ContextActionMenuPosition,
} from '@/components/ui/context-action-menu';
import { copyTextToClipboard } from '@/lib/utils';

import { AuthorAvatar } from './AuthorAvatar';
import { type MentionAuthorView } from './types';

type MentionHoverCardProps = {
  pubkey: string;
  label: string;
  author?: MentionAuthorView | null;
  children: ReactElement<MentionHoverCardTriggerProps>;
};

type MentionHoverCardTriggerProps = {
  className?: string;
  onContextMenu?: (event: ReactMouseEvent<HTMLElement>) => void;
  onKeyDown?: (event: ReactKeyboardEvent<HTMLElement>) => void;
};

export function MentionHoverCard({ pubkey, label, author, children }: MentionHoverCardProps) {
  const { t } = useTranslation('common');
  const displayLabel = author?.label?.trim() || label;
  const [identifierMenuPosition, setIdentifierMenuPosition] =
    useState<ContextActionMenuPosition | null>(null);
  const trigger = cloneElement(children, {
    className: [children.props.className, 'mention-context-target'].filter(Boolean).join(' '),
    onContextMenu: (event: ReactMouseEvent<HTMLElement>) => {
      children.props.onContextMenu?.(event);
      if (event.defaultPrevented) return;
      setIdentifierMenuPosition(contextActionMenuPositionFromPointer(event));
    },
    onKeyDown: (event: ReactKeyboardEvent<HTMLElement>) => {
      children.props.onKeyDown?.(event);
      if (event.defaultPrevented) return;
      const position = contextActionMenuPositionFromKeyboard(event);
      if (position) setIdentifierMenuPosition(position);
    },
  });

  return (
    <>
      <HoverCard openDelay={180} closeDelay={120}>
        <HoverCardTrigger asChild>
          {trigger}
        </HoverCardTrigger>
        <HoverCardContent className='mention-hover-card' align='start'>
          <div className='mention-hover-card-header'>
            <AuthorAvatar label={displayLabel} picture={author?.picture ?? null} size='lg' />
            <div className='mention-hover-card-identity'>
              <strong className='mention-hover-card-label'>{displayLabel}</strong>
              {author?.name?.trim() ? (
                <span className='mention-hover-card-name'>{author.name}</span>
              ) : null}
            </div>
          </div>
          {author?.aboutPreview?.trim() ? (
            <p className='mention-hover-card-about'>{author.aboutPreview}</p>
          ) : null}
        </HoverCardContent>
      </HoverCard>
      <ContextActionMenu
        open={identifierMenuPosition !== null}
        position={identifierMenuPosition}
        items={[
          {
            id: 'copy-author-id',
            label: t('actions.copyAuthorId'),
            onSelect: async () => {
              await copyTextToClipboard(pubkey);
            },
          },
        ]}
        onClose={() => setIdentifierMenuPosition(null)}
      />
    </>
  );
}
