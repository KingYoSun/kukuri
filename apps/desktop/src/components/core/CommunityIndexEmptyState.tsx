import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';

import { topicDisplayName } from '@/lib/topicId';

import type { CommunityIndexingTarget } from './CommunityIndexingRequestDialog';
import type {
  CommunityIndexEmptyAction,
  CommunityIndexEmptyGuidance,
  CommunityIndexEmptyIndexStatus,
} from './communityIndexEmptyGuidance';

type CommunityIndexEmptyStateProps = {
  guidance: CommunityIndexEmptyGuidance;
  nodeBaseUrl: string;
  retryDisabled?: boolean;
  onRetry: () => void;
  onOpenAuthor: (pubkey: string) => void;
  onOpenTimeline?: () => void;
  onRequestIndexing?: (target: CommunityIndexingTarget) => void;
  onOpenCommunityNodeSettings: () => void;
  onOpenConnectivitySettings?: () => void;
};

/**
 * 検索成功 0 件の空状態(#960)。何をどこで探したか、0 件の考えられる理由、次の行動を同じ場所に置く。
 * #975: 選択ノードでの索引状況(確定 / 取得中 / 未確認)を区別して示す。取得は workspace が行い、
 * この component は表示と既存導線の呼出しだけを行う。送信・認証・同意・保存は起こさない。
 */
export function CommunityIndexEmptyState({
  guidance,
  nodeBaseUrl,
  retryDisabled = false,
  onRetry,
  onOpenAuthor,
  onOpenTimeline,
  onRequestIndexing,
  onOpenCommunityNodeSettings,
  onOpenConnectivitySettings,
}: CommunityIndexEmptyStateProps) {
  const { t } = useTranslation(['shell']);
  const titleId = useId();

  const renderIndexStatus = (status: CommunityIndexEmptyIndexStatus) => {
    const targetNoun = t(`shell:communityIndex.emptyState.indexStatus.targetNoun.${guidance.scope === 'channel' ? 'channel' : 'topic'}`);
    switch (status.kind) {
      case 'loading':
        return <p className='text-sm'>{t('shell:communityIndex.emptyState.indexStatus.loading')}</p>;
      case 'unknown':
        return <p className='text-sm'>{t('shell:communityIndex.emptyState.indexStatus.unknown')}</p>;
      case 'privateUnverified':
        return <p className='text-sm'>{t('shell:communityIndex.emptyState.indexStatus.privateUnverified')}</p>;
      case 'target':
        return (
          <p className='text-sm font-medium' data-testid='community-index-empty-index-status'>
            {t(`shell:communityIndex.emptyState.indexStatus.target.${status.state}`, { target: targetNoun })}
          </p>
        );
      case 'ownRequests': {
        const groups = (['approved', 'pending', 'rejected'] as const).filter((group) => status[group].length > 0);
        return (
          <div className='space-y-1 text-sm' data-testid='community-index-empty-index-status'>
            {groups.length === 0 ? (
              <p>{t('shell:communityIndex.emptyState.indexStatus.ownRequests.none')}</p>
            ) : (
              groups.map((group) => (
                <p key={group}>
                  {t(`shell:communityIndex.emptyState.indexStatus.ownRequests.${group}`, {
                    targets: status[group].map((target) => topicDisplayName(target)).join(', '),
                  })}
                </p>
              ))
            )}
          </div>
        );
      }
      default:
        return null;
    }
  };

  const renderAction = (action: CommunityIndexEmptyAction) => {
    switch (action) {
      case 'openAuthor':
        return guidance.authorPubkey ? (
          <Button key={action} type='button' onClick={() => onOpenAuthor(guidance.authorPubkey!)}>
            {t('shell:communityIndex.emptyState.actions.openAuthor')}
          </Button>
        ) : null;
      case 'retry':
        return (
          <Button key={action} type='button' variant='secondary' disabled={retryDisabled} onClick={onRetry}>
            {guidance.operation === 'search'
              ? t('shell:communityIndex.emptyState.actions.retrySearch')
              : t('shell:communityIndex.emptyState.actions.retry')}
          </Button>
        );
      case 'openTimeline':
        return onOpenTimeline ? (
          <Button key={action} type='button' variant='secondary' onClick={onOpenTimeline}>
            {t('shell:communityIndex.emptyState.actions.openTimeline')}
          </Button>
        ) : null;
      case 'requestIndexing':
        return onRequestIndexing && guidance.indexingTarget ? (
          <Button
            key={action}
            type='button'
            variant='secondary'
            onClick={() => onRequestIndexing(guidance.indexingTarget!)}
          >
            {t('shell:communityIndex.emptyState.actions.requestIndexing')}
          </Button>
        ) : null;
      case 'openCommunityNodeSettings':
        return (
          <Button key={action} type='button' variant='secondary' onClick={onOpenCommunityNodeSettings}>
            {t('shell:workspace.communityNodeUnavailableAction')}
          </Button>
        );
      case 'openConnectivity':
        return onOpenConnectivitySettings ? (
          <Button key={action} type='button' variant='secondary' onClick={onOpenConnectivitySettings}>
            {t('shell:communityIndex.emptyState.actions.openConnectivity')}
          </Button>
        ) : null;
      default:
        return null;
    }
  };

  return (
    <Notice
      className='shell-community-index-notice shell-community-index-empty'
      role='status'
      aria-labelledby={titleId}
      data-testid='community-index-empty-state'
    >
      <div className='space-y-2'>
        <p id={titleId} className='font-medium'>
          {t('shell:communityIndex.empty')}
        </p>
        <p className='break-all text-sm'>
          {t('shell:communityIndex.nodeLabel')}: {nodeBaseUrl}
        </p>
        <p className='text-sm'>{t(`shell:communityIndex.emptyState.scope.${guidance.scope}`)}</p>
        {guidance.explainsPostTextMatching ? (
          <p className='text-sm'>{t('shell:communityIndex.emptyState.matchesPostText')}</p>
        ) : null}
        {guidance.indexStatus ? renderIndexStatus(guidance.indexStatus) : null}
        <p className='text-sm font-medium'>{t('shell:communityIndex.emptyState.reasonsHeading')}</p>
        <ul className='list-disc space-y-1 pl-5 text-sm'>
          {guidance.reasons.map((reason) => (
            <li key={reason}>{t(`shell:communityIndex.emptyState.reasons.${reason}`)}</li>
          ))}
        </ul>
        {guidance.explainsTopicListRequest ? (
          <p className='text-sm'>{t('shell:communityIndex.emptyState.exploreRequestHint')}</p>
        ) : null}
        <div className='flex flex-wrap gap-2'>{guidance.actions.map(renderAction)}</div>
      </div>
    </Notice>
  );
}
