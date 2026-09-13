import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { MoreHorizontal } from 'lucide-react';

import { AuthorAvatar } from '@/components/core/AuthorAvatar';
import { BlockedActionTooltip } from '@/components/core/BlockedActionTooltip';
import { RelationshipBadge } from '@/components/core/RelationshipBadge';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { IconButton } from '@/components/ui/icon-button';
import {
  ContextActionMenu,
  contextActionMenuPositionFromKeyboard,
  contextActionMenuPositionFromPointer,
  type ContextActionMenuPosition,
} from '@/components/ui/context-action-menu';
import type { ProfileConnectionsView } from '@/components/shell/types';
import type { ExtendedPanelStatus } from '@/components/extended/types';
import type { AuthorSocialView } from '@/lib/api';
import { copyTextToClipboard } from '@/lib/utils';

type ProfileConnectionsPanelProps = {
  activeView: ProfileConnectionsView;
  items: Array<AuthorSocialView & { picture_src?: string | null }>;
  localAuthorPubkey: string;
  status: ExtendedPanelStatus;
  error: string | null;
  onSelectView: (view: ProfileConnectionsView) => void;
  onToggleRelationship: (authorPubkey: string, following: boolean) => void;
  onToggleMute: (authorPubkey: string, muted: boolean) => void;
  onToggleBlock: (authorPubkey: string, blocking: boolean) => void;
  onBack: () => void;
};

const CONNECTION_VIEWS: ProfileConnectionsView[] = ['following', 'followed', 'muted', 'blocking'];

function displayLabel(author: AuthorSocialView, unknownAuthorLabel: string): string {
  return author.display_name?.trim() || author.name?.trim() || unknownAuthorLabel;
}

function followBlocked(author: AuthorSocialView): boolean {
  return author.blocking && !author.following;
}

function strongestRelationshipLabel(author: AuthorSocialView): string | null {
  if (author.mutual) {
    return 'mutual';
  }
  if (author.following) {
    return 'following';
  }
  if (author.followed_by) {
    return 'follows you';
  }
  if (author.friend_of_friend) {
    return 'friend of friend';
  }
  return null;
}

export function ProfileConnectionsPanel({
  activeView,
  items,
  localAuthorPubkey,
  status,
  error,
  onSelectView,
  onToggleRelationship,
  onToggleMute,
  onToggleBlock,
  onBack,
}: ProfileConnectionsPanelProps) {
  const { t } = useTranslation(['profile', 'common']);
  const [identifierMenuPosition, setIdentifierMenuPosition] =
    useState<ContextActionMenuPosition | null>(null);
  const [identifierAuthorPubkey, setIdentifierAuthorPubkey] = useState<string | null>(null);
  const [actionMenu, setActionMenu] = useState<{
    authorPubkey: string;
    position: ContextActionMenuPosition;
  } | null>(null);
  const primaryAction = activeView === 'blocking' ? 'block' : activeView === 'muted' ? 'mute' : 'follow';
  const menuAuthor = items.find((author) => author.author_pubkey === actionMenu?.authorPubkey);
  const actionMenuItems = useMemo(() => {
    if (!menuAuthor || menuAuthor.author_pubkey === localAuthorPubkey) return [];
    const author = menuAuthor;
    return [
      { id: 'follow', label: t(author.following ? 'common:actions.unfollow' : 'common:actions.follow'),
        // #992: ブロック中はフォローだけ無効。解除は残す。
        disabled: followBlocked(author),
        onSelect: () => onToggleRelationship(author.author_pubkey, author.following) },
      { id: 'mute', label: t(author.muted ? 'common:actions.unmute' : 'common:actions.mute'),
        onSelect: () => onToggleMute(author.author_pubkey, author.muted) },
      { id: 'block', label: t(author.blocking ? 'common:actions.unblock' : 'common:actions.block'),
        onSelect: () => onToggleBlock(author.author_pubkey, author.blocking) },
    ].filter((action) => action.id !== primaryAction);
  }, [localAuthorPubkey, menuAuthor, onToggleBlock, onToggleMute, onToggleRelationship, primaryAction, t]);
  const identifierMenuItems = useMemo(
    () =>
      identifierAuthorPubkey
        ? [
            {
              id: 'copy-author-id',
              label: t('common:actions.copyAuthorId'),
              onSelect: async () => {
                await copyTextToClipboard(identifierAuthorPubkey);
              },
            },
          ]
        : [],
    [identifierAuthorPubkey, t]
  );

  return (
    <Card className='panel-subsection profile-connections-panel'>
      <CardHeader className='profile-connections-toolbar'>
        <Button variant='secondary' type='button' onClick={onBack}>
          {t('connections.back')}
        </Button>
      </CardHeader>

      <div
        className='shell-workspace-tabs shell-profile-connections-tabs'
        role='tablist'
        aria-label={t('connections.tabsLabel')}
      >
        {CONNECTION_VIEWS.map((view) => (
          <button
            key={view}
            className={`shell-tab${activeView === view ? ' shell-tab-active' : ''}`}
            role='tab'
            type='button'
            aria-selected={activeView === view}
            onClick={() => { setActionMenu(null); onSelectView(view); }}
          >
            {t(`connections.tabs.${view}`)}
          </button>
        ))}
      </div>

      {status === 'loading' ? <Notice>{t('connections.loading')}</Notice> : null}
      {status === 'error' && error ? <Notice tone='destructive'>{error}</Notice> : null}

      {status === 'ready' && items.length === 0 ? (
        <p className='empty-state'>{t(`connections.empty.${activeView}`)}</p>
      ) : null}

      {items.length > 0 ? (
        <ul className='post-list'>
          {items.map((author) => {
            const label = displayLabel(author, t('common:fallbacks.unknownAuthor'));
            const relationshipLabel = strongestRelationshipLabel(author);
            const showActions = author.author_pubkey !== localAuthorPubkey;
            const primaryButton = (
              <Button
                variant='secondary'
                type='button'
                className='profile-connection-primary-action'
                onClick={() => {
                  if (primaryAction === 'block') onToggleBlock(author.author_pubkey, author.blocking);
                  else if (primaryAction === 'mute') onToggleMute(author.author_pubkey, author.muted);
                  else onToggleRelationship(author.author_pubkey, author.following);
                }}
              >
                {primaryAction === 'block'
                  ? t(author.blocking ? 'common:actions.unblock' : 'common:actions.block')
                  : primaryAction === 'mute'
                    ? t(author.muted ? 'common:actions.unmute' : 'common:actions.mute')
                    : t(author.following ? 'common:actions.unfollow' : 'common:actions.follow')}
              </Button>
            );

            return (
              <li key={author.author_pubkey}>
                <article
                  className='post-card profile-connection-card'
                  tabIndex={0}
                  data-testid='profile-connection-identifier-target'
                  onContextMenu={(event) => {
                    setActionMenu(null);
                    setIdentifierAuthorPubkey(author.author_pubkey);
                    setIdentifierMenuPosition(contextActionMenuPositionFromPointer(event));
                  }}
                  onKeyDown={(event) => {
                    const position = contextActionMenuPositionFromKeyboard(event);
                    if (position) {
                      setActionMenu(null);
                      setIdentifierAuthorPubkey(author.author_pubkey);
                      setIdentifierMenuPosition(position);
                    }
                  }}
                >
                  <div className='profile-connection-header'>
                    <div className='profile-connection-badges'>
                      {relationshipLabel ? <RelationshipBadge label={relationshipLabel} /> : null}
                      {author.muted ? (
                        <span className='relationship-badge relationship-badge-direct'>
                          {t('connections.mutedBadge')}
                        </span>
                      ) : null}
                      {author.blocking ? (
                        <span className='relationship-badge relationship-badge-direct'>
                          {t('connections.blockedBadge')}
                        </span>
                      ) : null}
                    </div>
                    {showActions ? (
                      <IconButton
                        variant='ghost'
                        label={t('connections.actionsFor', { name: label })}
                        aria-haspopup='menu'
                        aria-expanded={actionMenu?.authorPubkey === author.author_pubkey}
                        onClick={(event) => {
                          const trigger = event.currentTarget;
                          const rect = trigger.getBoundingClientRect();
                          setIdentifierMenuPosition(null);
                          setActionMenu({ authorPubkey: author.author_pubkey,
                            position: { x: rect.right, y: rect.bottom, returnFocusTo: trigger } });
                        }}
                      >
                        <MoreHorizontal aria-hidden='true' className='size-4' />
                      </IconButton>
                    ) : null}
                  </div>
                  <div className='profile-connection-info'>
                    <AuthorAvatar label={label} picture={author.picture_src ?? null} size='sm' />
                    <div className='author-detail-copy-stack'>
                      <strong className='post-title'>{label}</strong>
                      <p className='author-detail-copy author-detail-break'>
                        {author.about?.trim() || t('common:fallbacks.noBio')}
                      </p>
                    </div>
                    {showActions ? (
                      primaryAction === 'follow' && followBlocked(author) ? (
                        <BlockedActionTooltip reason={t('common:relationships.blockedFollowReason')}>
                          {primaryButton}
                        </BlockedActionTooltip>
                      ) : (
                        primaryButton
                      )
                    ) : null}
                  </div>
                </article>
              </li>
            );
          })}
        </ul>
      ) : null}
      <ContextActionMenu
        open={identifierMenuPosition !== null && identifierMenuItems.length > 0}
        position={identifierMenuPosition}
        items={identifierMenuItems}
        onClose={() => {
          setIdentifierMenuPosition(null);
          setIdentifierAuthorPubkey(null);
        }}
      />
      <ContextActionMenu
        open={actionMenu !== null && actionMenuItems.length > 0}
        position={actionMenu?.position ?? null}
        items={actionMenuItems}
        onClose={() => setActionMenu(null)}
      />
    </Card>
  );
}
