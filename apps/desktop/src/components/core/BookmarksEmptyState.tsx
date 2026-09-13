import { type ReactNode, useId } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { Bookmark } from 'lucide-react';

import { ActionRef } from '@/components/ui/action-ref';
import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import type { ExtendedPanelStatus } from '@/components/extended/types';
import { useMinimumLoading } from '@/lib/useMinimumLoading';

import { EmptyStateGuidance } from './EmptyStateGuidance';

type BookmarksEmptyStateProps = {
  /** 同じ Column の表示をフィードへ戻す既存導線。無い場合は CTA を出さない。 */
  onShowTimeline?: () => void;
};

/** ブックマーク一覧の取得成功 0 件(#994)。投稿カードのブックマーク icon ボタンを chip で示す。 */
export function BookmarksEmptyState({ onShowTimeline }: BookmarksEmptyStateProps) {
  const { t } = useTranslation(['shell', 'common']);
  return (
    <EmptyStateGuidance
      testId='bookmarks-empty-state'
      title={t('shell:workspace.noBookmarks')}
      steps={[
        {
          id: 'bookmark',
          content: (
            <Trans
              i18nKey='shell:workspace.bookmarksEmpty.step'
              components={{ action: <ActionRef icon={Bookmark} label={t('common:actions.bookmark')} /> }}
            />
          ),
        },
      ]}
      notes={[{ id: 'local', content: t('shell:workspace.bookmarksEmpty.note') }]}
      actions={
        onShowTimeline
          ? [{ id: 'timeline', label: t('shell:workspace.bookmarksEmpty.showTimeline'), onClick: onShowTimeline }]
          : []
      }
    />
  );
}

type BookmarksListFrameProps = {
  status: ExtendedPanelStatus;
  error: string | null;
  onRetry: () => void;
  /** 表示用 status(最低 loading 適用後)を受け取り、一覧本体を描画する。 */
  children: (displayedStatus: ExtendedPanelStatus) => ReactNode;
};

/**
 * ブックマーク一覧の loading / error 通知(#994)。初回取得は最低時間 loading を保ち、
 * 値がある場合は一覧を消さずに通知だけを重ねる。再試行は既存 loader を 1 回呼ぶだけ。
 */
export function BookmarksListFrame({ status, error, onRetry, children }: BookmarksListFrameProps) {
  const { t } = useTranslation(['shell', 'common']);
  const loadingId = useId();
  const displayedStatus = useMinimumLoading(status);
  return (
    <>
      {displayedStatus === 'loading' ? (
        <Notice className='shell-empty-guidance' role='status' aria-labelledby={loadingId}>
          <p id={loadingId}>{t('shell:workspace.bookmarksLoading')}</p>
        </Notice>
      ) : null}
      {displayedStatus === 'error' ? (
        <Notice className='shell-empty-guidance' tone='destructive' role='alert'>
          <div className='space-y-2'>
            <p>{error ?? t('common:errors.failedToLoadBookmarks')}</p>
            <Button type='button' variant='secondary' onClick={onRetry}>
              {t('common:actions.retry')}
            </Button>
          </div>
        </Notice>
      ) : null}
      {children(displayedStatus)}
    </>
  );
}
