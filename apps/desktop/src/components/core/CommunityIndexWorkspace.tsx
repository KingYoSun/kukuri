import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { TFunction } from 'i18next';
import { Search } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type {
  AuthorSocialView,
  CommunityNodeIndexQueryRequest,
  DesktopApi,
  IndexEntryView,
  TimelineScope,
} from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';
import { copyTextToClipboard } from '@/lib/utils';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import { communityIndexPostCardView } from './communityIndexPostCardView';
import { PostCard } from './PostCard';

type IndexOperation = 'search' | 'discovery' | 'recommendations';

type CommunityIndexWorkspaceProps = {
  api: DesktopApi;
  mode: 'topic' | 'explore';
  activeTopic: string;
  activeTimelineScope: TimelineScope;
  eligibleNodeBaseUrls: readonly string[];
  selectedNodeBaseUrl: string | null;
  onOpenCommunityNodeSettings: () => void;
  knownAuthorsByPubkey?: Record<string, AuthorSocialView>;
  mediaObjectUrls?: Record<string, string | null>;
  onOpenAuthor: (pubkey: string) => void;
};

type IndexRequestContext = {
  key: string;
  nodeBaseUrl: string;
  operation: IndexOperation;
  scopeKind: CommunityNodeIndexQueryRequest['scope_kind'];
  scopeId: CommunityNodeIndexQueryRequest['scope_id'];
  topicId: CommunityNodeIndexQueryRequest['topic_id'];
};

type IndexResultState = {
  context: IndexRequestContext;
  entries: IndexEntryView[];
};

function operationMethod(api: DesktopApi, operation: IndexOperation) {
  if (operation === 'search') return api.searchCommunityNodeIndex.bind(api);
  if (operation === 'discovery') return api.discoverCommunityNodeIndex.bind(api);
  return api.recommendCommunityNodeIndex.bind(api);
}

function communityIndexErrorMessage(error: unknown, t: TFunction): string {
  if (!(error instanceof InvokeError)) {
    return error instanceof Error ? error.message : String(error);
  }
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) {
    return t('shell:communityIndex.errors.authRequired');
  }
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) {
    return t('shell:communityIndex.errors.consentRequired');
  }
  if (error.code === 'INDEX_QUERY_NOT_CONFIGURED') {
    return t('shell:communityIndex.errors.notConfigured');
  }
  if (error.code === 'INDEX_QUERY_NOT_ACTIVATED') {
    return t('shell:communityIndex.errors.notActivated');
  }
  if (error.status === 429 || error.code === 'RATE_LIMITED') {
    return error.retryAfterSeconds
      ? t('shell:communityIndex.errors.rateLimitedWithRetry', {
          seconds: error.retryAfterSeconds,
        })
      : t('shell:communityIndex.errors.rateLimited');
  }
  return error.message;
}

function indexContext(
  mode: CommunityIndexWorkspaceProps['mode'],
  operation: IndexOperation,
  selectedNodeBaseUrl: string | null,
  activeTopic: string,
  activeTimelineScope: TimelineScope
): IndexRequestContext | null {
  if (!selectedNodeBaseUrl) return null;
  const scopeKind =
    mode !== 'topic'
      ? null
      : activeTimelineScope.kind === 'channel'
        ? 'private_channel'
        : 'public_topic';
  const scopeId =
    mode !== 'topic'
      ? null
      : activeTimelineScope.kind === 'channel'
        ? activeTimelineScope.channel_id
        : activeTopic;
  return {
    key: [
      selectedNodeBaseUrl,
      operation,
      mode,
      activeTimelineScope.kind,
      scopeKind ?? '',
      scopeId ?? '',
    ].join('\u0000'),
    nodeBaseUrl: selectedNodeBaseUrl,
    operation,
    scopeKind,
    scopeId,
    // 非公開チャンネル範囲では所属証明(channel secret)を引くために topic が要る(#711)。
    topicId: scopeKind === 'private_channel' ? activeTopic : null,
  };
}

export function CommunityIndexWorkspace({
  api,
  mode,
  activeTopic,
  activeTimelineScope,
  eligibleNodeBaseUrls,
  selectedNodeBaseUrl,
  onOpenCommunityNodeSettings,
  knownAuthorsByPubkey = {},
  mediaObjectUrls = {},
  onOpenAuthor,
}: CommunityIndexWorkspaceProps) {
  const { t } = useTranslation(['shell', 'common']);
  const [operation, setOperation] = useState<IndexOperation>('search');
  const [query, setQuery] = useState('');
  const [resultState, setResultState] = useState<IndexResultState | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [error, setError] = useState<string | null>(null);
  const requestSequence = useRef(0);

  const effectiveOperation: IndexOperation = mode === 'topic' ? 'search' : operation;
  const isAllJoined = mode === 'topic' && activeTimelineScope.kind === 'all_joined';
  // 選択値が適格一覧(認証・同意・通信・提供中能力)に含まれない間は、再調整が追いつくまで
  // 古いノードへ要求を送らない(#698)。文脈も無効化するので古い結果・通報対象は失効する。
  const activeNodeBaseUrl =
    selectedNodeBaseUrl !== null && eligibleNodeBaseUrls.includes(selectedNodeBaseUrl)
      ? selectedNodeBaseUrl
      : null;
  const disabled = activeNodeBaseUrl === null || isAllJoined;
  const currentContext = useMemo(
    () => indexContext(mode, effectiveOperation, activeNodeBaseUrl, activeTopic, activeTimelineScope),
    [activeNodeBaseUrl, activeTimelineScope, activeTopic, effectiveOperation, mode]
  );
  const currentContextKey = currentContext?.key ?? null;
  const currentContextKeyRef = useRef(currentContextKey);
  const visibleResult =
    resultState && resultState.context.key === currentContextKey ? resultState : null;
  const visiblePostCards = useMemo(
    () =>
      visibleResult?.entries.map((entry) => ({
        key: `${entry.scope_kind}:${entry.scope_id}:${entry.object_id}`,
        view: communityIndexPostCardView(entry, {
          nodeBaseUrl: visibleResult.context.nodeBaseUrl,
          operation: visibleResult.context.operation,
          knownAuthor: knownAuthorsByPubkey[entry.author_pubkey] ?? null,
          mediaObjectUrls,
        }),
      })) ?? [],
    [knownAuthorsByPubkey, mediaObjectUrls, visibleResult]
  );

  const invalidateResults = useCallback(() => {
    requestSequence.current += 1;
    setStatus('idle');
    setResultState(null);
    setError(null);
  }, []);

  useEffect(() => {
    currentContextKeyRef.current = currentContextKey;
    invalidateResults();
  }, [currentContextKey, invalidateResults]);

  async function runQuery(event?: FormEvent) {
    event?.preventDefault();
    if (disabled || !currentContext) return;
    if (effectiveOperation === 'search' && !query.trim()) {
      setStatus('error');
      setError(t('shell:communityIndex.queryRequired'));
      return;
    }
    const request: CommunityNodeIndexQueryRequest = {
      base_url: currentContext.nodeBaseUrl,
      query: currentContext.operation === 'search' ? query.trim() : null,
      scope_kind: currentContext.scopeKind,
      scope_id: currentContext.scopeId,
      topic_id: currentContext.topicId,
      limit: 50,
    };
    const requestContext = currentContext;
    const sequence = ++requestSequence.current;
    setStatus('loading');
    setError(null);
    try {
      const response = await operationMethod(api, requestContext.operation)(request);
      if (
        sequence !== requestSequence.current ||
        requestContext.key !== currentContextKeyRef.current
      ) {
        return;
      }
      setResultState({ context: requestContext, entries: response.entries });
      setStatus('success');
    } catch (cause) {
      if (
        sequence !== requestSequence.current ||
        requestContext.key !== currentContextKeyRef.current
      ) {
        return;
      }
      setResultState(null);
      setError(communityIndexErrorMessage(cause, t));
      setStatus('error');
    }
  }

  return (
    <Card className='shell-workspace-card space-y-4' data-testid={`community-index-${mode}`}>
      <div className='flex flex-wrap items-start justify-between gap-3'>
        <div className='space-y-1'>
          <h3 className='text-lg font-semibold'>{t('shell:communityIndex.title')}</h3>
          <p className='text-sm text-[var(--muted-foreground)]'>
            {mode === 'topic'
              ? t('shell:communityIndex.topicSummary')
              : t('shell:communityIndex.exploreSummary')}
          </p>
        </div>
      </div>

      {mode === 'explore' ? (
        <div
          className='shell-workspace-tabs shell-community-index-tabs'
          role='tablist'
          aria-label={t('shell:communityIndex.surfaces')}
        >
          {(['search', 'discovery', 'recommendations'] as const).map((value) => (
            <button
              key={value}
              className={`shell-tab${operation === value ? ' shell-tab-active' : ''}`}
              type='button'
              role='tab'
              aria-selected={operation === value}
              onClick={() => {
                invalidateResults();
                setOperation(value);
              }}
            >
              {t(`shell:communityIndex.operations.${value}`)}
            </button>
          ))}
        </div>
      ) : null}

      {eligibleNodeBaseUrls.length === 0 ? (
        <Notice tone='warning'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <span>{t('shell:communityIndex.noEligibleNode')}</span>
            <Button variant='secondary' type='button' onClick={onOpenCommunityNodeSettings}>
              {t('shell:workspace.communityNodeUnavailableAction')}
            </Button>
          </div>
        </Notice>
      ) : activeNodeBaseUrl === null ? (
        <Notice tone='warning'>
          <div className='flex flex-wrap items-center justify-between gap-3'>
            <span>{t('shell:communityIndex.selectedNodeUnavailable')}</span>
            <Button variant='secondary' type='button' onClick={onOpenCommunityNodeSettings}>
              {t('shell:workspace.communityNodeUnavailableAction')}
            </Button>
          </div>
        </Notice>
      ) : isAllJoined ? (
        <Notice tone='warning'>{t('shell:communityIndex.allJoinedDisabled')}</Notice>
      ) : (
        <form className='flex flex-col gap-3 sm:flex-row' onSubmit={(event) => void runQuery(event)}>
          {effectiveOperation === 'search' ? (
            <Input
              aria-label={t('shell:communityIndex.queryLabel')}
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
              placeholder={t('shell:communityIndex.queryPlaceholder')}
            />
          ) : (
            <p className='flex-1 self-center text-sm text-[var(--muted-foreground)]'>
              {t(`shell:communityIndex.operationHints.${effectiveOperation}`)}
            </p>
          )}
          <Button type='submit' disabled={status === 'loading'}>
            <Search className='size-4' aria-hidden='true' />
            {status === 'loading'
              ? t('shell:communityIndex.loading')
              : t('shell:communityIndex.run')}
          </Button>
        </form>
      )}

      {error ? <Notice tone='destructive'>{error}</Notice> : null}
      {status === 'success' && visibleResult && visibleResult.entries.length === 0 ? (
        <p className='empty-state'>{t('shell:communityIndex.empty')}</p>
      ) : null}
      {visiblePostCards.length > 0 ? (
        <ul className='post-list' aria-label={t('shell:communityIndex.results')}>
          {visiblePostCards.map(({ key, view }) => (
            <li key={key}>
              <PostCard
                view={view}
                readOnly
                mediaObjectUrls={mediaObjectUrls}
                onOpenAuthor={onOpenAuthor}
                onOpenThread={() => undefined}
                onReply={() => undefined}
                onSubmitReport={(request) => api.submitCommunityNodeReport(request)}
                onCopyReportContact={(value) => void copyTextToClipboard(value)}
                onFetchReportManifest={(baseUrl) => api.fetchCommunityNodeManifest(baseUrl)}
              />
            </li>
          ))}
        </ul>
      ) : null}
    </Card>
  );
}
