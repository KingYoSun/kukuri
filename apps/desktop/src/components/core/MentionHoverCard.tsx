import type { ReactNode } from 'react';

import { HoverCard, HoverCardContent, HoverCardTrigger } from '@/components/ui/hover-card';
import { shortPubkey } from '@/shell/selectors';

import { AuthorAvatar } from './AuthorAvatar';
import { type MentionAuthorView } from './types';

type MentionHoverCardProps = {
  pubkey: string;
  label: string;
  author?: MentionAuthorView | null;
  children: ReactNode;
};

export function MentionHoverCard({ pubkey, label, author, children }: MentionHoverCardProps) {
  const displayLabel = author?.label?.trim() || label;

  return (
    <HoverCard openDelay={180} closeDelay={120}>
      <HoverCardTrigger asChild>{children}</HoverCardTrigger>
      <HoverCardContent className='mention-hover-card' align='start'>
        <div className='mention-hover-card-header'>
          <AuthorAvatar label={displayLabel} picture={author?.picture ?? null} size='lg' />
          <div className='mention-hover-card-identity'>
            <strong className='mention-hover-card-label'>{displayLabel}</strong>
            {author?.name?.trim() ? (
              <span className='mention-hover-card-name'>{author.name}</span>
            ) : null}
            <span className='mention-hover-card-pubkey'>{shortPubkey(pubkey)}</span>
          </div>
        </div>
        {author?.aboutPreview?.trim() ? (
          <p className='mention-hover-card-about'>{author.aboutPreview}</p>
        ) : null}
      </HoverCardContent>
    </HoverCard>
  );
}
