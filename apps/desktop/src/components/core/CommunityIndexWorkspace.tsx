import { type FormEvent, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { TFunction } from 'i18next';
import { Search, ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type {
  CommunityNodeManifest,
  CommunityNodeIndexQueryRequest,
  DesktopApi,
  IndexEntryView,
  TimelineScope,
} from '@/lib/api';
import { formatLocalizedTime } from '@/i18n/format';
import type { SupportedLocale } from '@/i18n';
import { InvokeError } from '@/lib/api/invoke/error';
import { planReportRouting } from '@/lib/api/reportRouting';
import { copyTextToClipboard } from '@/lib/utils';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import {
  ContextActionMenu,
  contextActionMenuPositionFromKeyboard,
  contextActionMenuPositionFromPointer,
  type ContextActionMenuPosition,
} from '@/components/ui/context-action-menu';
import { ReportRoutingDialog } from './ReportRoutingDialog';

type IndexOperation = 'search' | 'discovery' | 'recommendations';

type CommunityIndexWorkspaceProps = {
  api: DesktopApi;
  mode: 'topic' | 'explore';
  locale: SupportedLocale;
  activeTopic: string;
  activeTimelineScope: TimelineScope;
  eligibleNodeBaseUrls: readonly string[];
  selectedNodeBaseUrl: string | null;
  onOpenCommunityNodeSettings: () => void;
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

type ReportSelection = {
  context: IndexRequestContext;
  entry: IndexEntryView;
};

const INDEX_REPORT_IDENTITY = {
  capability: 'community_index',
  subjectKind: 'search_result',
} as const;

const RECOMMENDATION_REPORT_IDENTITY = {
  capability: 'recommendation',
  subjectKind: 'recommendation',
} as const;

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

function reportIdentity(operation: IndexOperation) {
  return operation === 'recommendations'
    ? RECOMMENDATION_REPORT_IDENTITY
    : INDEX_REPORT_IDENTITY;
}

export function CommunityIndexWorkspace({
  api,
  mode,
  locale,
  activeTopic,
  activeTimelineScope,
  eligibleNodeBaseUrls,
  selectedNodeBaseUrl,
  onOpenCommunityNodeSettings,
}: CommunityIndexWorkspaceProps) {
  const { t } = useTranslation(['shell', 'common']);
  const [operation, setOperation] = useState<IndexOperation>('search');
  const [query, setQuery] = useState('');
  const [resultState, setResultState] = useState<IndexResultState | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [error, setError] = useState<string | null>(null);
  const [reportSelection, setReportSelection] = useState<ReportSelection | null>(null);
  const [reportManifest, setReportManifest] = useState<CommunityNodeManifest | null>(null);
  const [reportResolving, setReportResolving] = useState(false);
  const [reportResolveError, setReportResolveError] = useState<string | null>(null);
  const [identifierMenuPosition, setIdentifierMenuPosition] =
    useState<ContextActionMenuPosition | null>(null);
  const [identifierEntry, setIdentifierEntry] = useState<IndexEntryView | null>(null);
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
  const activeReportSelection =
    reportSelection && reportSelection.context.key === currentContextKey ? reportSelection : null;

  const invalidateResults = useCallback(() => {
    requestSequence.current += 1;
    setStatus('idle');
    setResultState(null);
    setError(null);
    setReportSelection(null);
  }, []);

  useEffect(() => {
    currentContextKeyRef.current = currentContextKey;
    invalidateResults();
  }, [currentContextKey, invalidateResults]);

  useEffect(() => {
    if (!activeReportSelection) {
      setReportManifest(null);
      setReportResolving(false);
      setReportResolveError(null);
      return;
    }
    let active = true;
    setReportManifest(null);
    setReportResolving(true);
    setReportResolveError(null);
    void api
      .fetchCommunityNodeManifest(activeReportSelection.context.nodeBaseUrl)
      .then((response) => {
        if (!active) return;
        if (response.status === 'ok' && response.manifest) {
          setReportManifest(response.manifest);
        } else {
          setReportResolveError(t('shell:report.resolveFailed'));
        }
      })
      .catch(() => {
        if (active) setReportResolveError(t('shell:report.resolveFailed'));
      })
      .finally(() => {
        if (active) setReportResolving(false);
      });
    return () => {
      active = false;
    };
  }, [activeReportSelection, api, t]);

  const activeReportIdentity = activeReportSelection
    ? reportIdentity(activeReportSelection.context.operation)
    : null;
  const reportPlan = useMemo(() => {
    if (!activeReportSelection || !activeReportIdentity) {
      return planReportRouting(null, {});
    }
    const sourceNodeBaseUrl = activeReportSelection.context.nodeBaseUrl;
    return planReportRouting(
      {
        canonicalSource: 'unknown',
        observedVia: [
          { nodeBaseUrl: sourceNodeBaseUrl, capability: activeReportIdentity.capability },
        ],
        responsibleReportTargets: [],
      },
      reportManifest ? { [sourceNodeBaseUrl]: reportManifest } : {}
    );
  }, [activeReportIdentity, activeReportSelection, reportManifest]);
  const identifierMenuItems = useMemo(
    () =>
      identifierEntry
        ? [
            {
              id: 'copy-author-id',
              label: t('common:actions.copyAuthorId'),
              onSelect: async () => {
                await copyTextToClipboard(identifierEntry.author_pubkey);
              },
            },
            {
              id: 'copy-post-id',
              label: t('common:actions.copyPostId'),
              onSelect: async () => {
                await copyTextToClipboard(identifierEntry.object_id);
              },
            },
          ]
        : [],
    [identifierEntry, t]
  );

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
      {visibleResult && visibleResult.entries.length > 0 ? (
        <ul className='space-y-3' aria-label={t('shell:communityIndex.results')}>
          {visibleResult.entries.map((entry) => (
            <li key={`${entry.scope_kind}:${entry.scope_id}:${entry.object_id}`}>
              <article
                className='rounded-[18px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4'
                tabIndex={0}
                onContextMenu={(event) => {
                  setIdentifierEntry(entry);
                  setIdentifierMenuPosition(contextActionMenuPositionFromPointer(event));
                }}
                onKeyDown={(event) => {
                  const position = contextActionMenuPositionFromKeyboard(event);
                  if (position) {
                    setIdentifierEntry(entry);
                    setIdentifierMenuPosition(position);
                  }
                }}
              >
                <div className='flex flex-wrap items-center gap-2 text-xs text-[var(--muted-foreground)]'>
                  <Badge tone='neutral'>{entry.scope_kind}</Badge>
                  <span>{entry.scope_id}</span>
                  <span>{formatLocalizedTime(entry.created_at, locale)}</span>
                </div>
                <p className='mt-3 whitespace-pre-wrap break-words text-sm'>{entry.text}</p>
                <p className='mt-2 text-xs text-[var(--muted-foreground)]'>
                  {t('shell:communityIndex.previewNotice')}
                </p>
                <div className='mt-3 flex flex-wrap items-center justify-end gap-2 text-xs'>
                  <Button
                    variant='secondary'
                    type='button'
                    onClick={() => setReportSelection({ context: visibleResult.context, entry })}
                  >
                    <ShieldAlert className='size-4' aria-hidden='true' />
                    {t('shell:report.actionLabel')}
                  </Button>
                </div>
              </article>
            </li>
          ))}
        </ul>
      ) : null}

      <ContextActionMenu
        open={identifierMenuPosition !== null && identifierMenuItems.length > 0}
        position={identifierMenuPosition}
        items={identifierMenuItems}
        onClose={() => {
          setIdentifierMenuPosition(null);
          setIdentifierEntry(null);
        }}
      />

      {activeReportSelection && activeReportIdentity ? (
        <ReportRoutingDialog
          open={true}
          onOpenChange={(open) => { if (!open) setReportSelection(null); }}
          subject={{
            kind: activeReportIdentity.subjectKind,
            id: activeReportSelection.entry.object_id,
            label: activeReportSelection.entry.scope_id,
          }}
          plan={reportPlan}
          resolving={reportResolving}
          resolveError={reportResolveError}
          onCopyContact={(value) => void copyTextToClipboard(value)}
          onSubmit={async ({ candidate, reason, details, reporterContact }) => {
            if (candidate.contact.kind !== 'endpoint') {
              throw new Error(t('shell:report.resolveFailed'));
            }
            return api.submitCommunityNodeReport({
              node_base_url: activeReportSelection.context.nodeBaseUrl,
              report_endpoint: candidate.contact.value,
              subject_kind: activeReportIdentity.subjectKind,
              subject_id: activeReportSelection.entry.object_id,
              capability: activeReportIdentity.capability,
              reason,
              details: details || null,
              reporter_contact: reporterContact || null,
            });
          }}
        />
      ) : null}
    </Card>
  );
}
