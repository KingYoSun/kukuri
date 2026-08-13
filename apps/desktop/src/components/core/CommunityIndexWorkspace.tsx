import { type FormEvent, useMemo, useRef, useState } from 'react';
import { Search, ShieldAlert } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import type {
  CommunityNodeIndexQueryRequest,
  DesktopApi,
  IndexEntryView,
  TimelineScope,
} from '@/lib/api';
import type { CommunityIndexManifestEntry } from '@/lib/api/communityIndex';
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
  onSelectNode: (baseUrl: string) => void;
  manifests: Readonly<Record<string, CommunityIndexManifestEntry>>;
  onOpenCommunityNodeSettings: () => void;
};

function operationMethod(api: DesktopApi, operation: IndexOperation) {
  if (operation === 'search') return api.searchCommunityNodeIndex.bind(api);
  if (operation === 'discovery') return api.discoverCommunityNodeIndex.bind(api);
  return api.recommendCommunityNodeIndex.bind(api);
}

function communityIndexErrorMessage(error: unknown): string {
  if (!(error instanceof InvokeError)) {
    return error instanceof Error ? error.message : String(error);
  }
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) {
    return 'Authentication is required for the selected Community Node.';
  }
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) {
    return 'Consent is required for the selected Community Node.';
  }
  if (error.code === 'INDEX_QUERY_NOT_CONFIGURED') {
    return 'Community Index is not configured on the selected node.';
  }
  if (error.code === 'INDEX_QUERY_NOT_ACTIVATED') {
    return 'Community Index is temporarily unavailable on the selected node.';
  }
  if (error.status === 429 || error.code === 'RATE_LIMITED') {
    return error.retryAfterSeconds
      ? `Too many requests. Try again in ${error.retryAfterSeconds} seconds.`
      : 'Too many requests. Please try again later.';
  }
  return error.message;
}

export function CommunityIndexWorkspace({
  api,
  mode,
  locale,
  activeTopic,
  activeTimelineScope,
  eligibleNodeBaseUrls,
  selectedNodeBaseUrl,
  onSelectNode,
  manifests,
  onOpenCommunityNodeSettings,
}: CommunityIndexWorkspaceProps) {
  const { t } = useTranslation(['shell', 'common']);
  const [operation, setOperation] = useState<IndexOperation>('search');
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<IndexEntryView[]>([]);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const [error, setError] = useState<string | null>(null);
  const [reportEntry, setReportEntry] = useState<IndexEntryView | null>(null);
  const requestSequence = useRef(0);

  const effectiveOperation: IndexOperation = mode === 'topic' ? 'search' : operation;
  const isAllJoined = mode === 'topic' && activeTimelineScope.kind === 'all_joined';
  const disabled = selectedNodeBaseUrl === null || isAllJoined;
  const selectedManifest = selectedNodeBaseUrl ? manifests[selectedNodeBaseUrl] : undefined;
  const reportPlan = useMemo(() => {
    if (!selectedNodeBaseUrl || selectedManifest?.status !== 'ok') {
      return planReportRouting(null, {});
    }
    return planReportRouting(
      {
        canonicalSource: 'unknown',
        observedVia: [{ nodeBaseUrl: selectedNodeBaseUrl, capability: 'community_index' }],
        responsibleReportTargets: [],
      },
      { [selectedNodeBaseUrl]: selectedManifest.manifest }
    );
  }, [selectedManifest, selectedNodeBaseUrl]);

  async function runQuery(event?: FormEvent) {
    event?.preventDefault();
    if (disabled || !selectedNodeBaseUrl) return;
    if (effectiveOperation === 'search' && !query.trim()) {
      setStatus('error');
      setError(t('shell:communityIndex.queryRequired'));
      return;
    }
    const request: CommunityNodeIndexQueryRequest = {
      base_url: selectedNodeBaseUrl,
      query: effectiveOperation === 'search' ? query.trim() : null,
      scope_kind:
        mode !== 'topic'
          ? null
          : activeTimelineScope.kind === 'channel'
            ? 'private_channel'
            : 'public_topic',
      scope_id:
        mode !== 'topic'
          ? null
          : activeTimelineScope.kind === 'channel'
            ? activeTimelineScope.channel_id
            : activeTopic,
      limit: 50,
    };
    const sequence = ++requestSequence.current;
    setStatus('loading');
    setError(null);
    try {
      const response = await operationMethod(api, effectiveOperation)(request);
      if (sequence !== requestSequence.current) return;
      setResults(response.entries);
      setStatus('success');
    } catch (cause) {
      if (sequence !== requestSequence.current) return;
      setResults([]);
      setError(communityIndexErrorMessage(cause));
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
        {eligibleNodeBaseUrls.length > 0 ? (
          <label className='flex min-w-56 flex-col gap-1 text-sm'>
            <span className='font-medium'>{t('shell:communityIndex.nodeLabel')}</span>
            <select
              className='rounded-lg border border-[var(--border-subtle)] bg-background px-3 py-2'
              value={selectedNodeBaseUrl ?? ''}
              onChange={(event) => onSelectNode(event.currentTarget.value)}
            >
              {eligibleNodeBaseUrls.map((baseUrl) => (
                <option key={baseUrl} value={baseUrl}>{baseUrl}</option>
              ))}
            </select>
          </label>
        ) : null}
      </div>

      {mode === 'explore' ? (
        <div className='shell-workspace-tabs' role='tablist' aria-label={t('shell:communityIndex.surfaces')}>
          {(['search', 'discovery', 'recommendations'] as const).map((value) => (
            <button
              key={value}
              className={`shell-tab${operation === value ? ' shell-tab-active' : ''}`}
              type='button'
              role='tab'
              aria-selected={operation === value}
              onClick={() => {
                requestSequence.current += 1;
                setOperation(value);
                setStatus('idle');
                setResults([]);
                setError(null);
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
      {status === 'success' && results.length === 0 ? (
        <p className='empty-state'>{t('shell:communityIndex.empty')}</p>
      ) : null}
      {results.length > 0 ? (
        <ul className='space-y-3' aria-label={t('shell:communityIndex.results')}>
          {results.map((entry) => (
            <li key={`${entry.scope_kind}:${entry.scope_id}:${entry.object_id}`}>
              <article className='rounded-[18px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4'>
                <div className='flex flex-wrap items-center gap-2 text-xs text-[var(--muted-foreground)]'>
                  <Badge tone='neutral'>{entry.scope_kind}</Badge>
                  <span>{entry.scope_id}</span>
                  <span>{formatLocalizedTime(entry.created_at, locale)}</span>
                </div>
                <p className='mt-3 whitespace-pre-wrap break-words text-sm'>{entry.text}</p>
                <p className='mt-2 text-xs text-[var(--muted-foreground)]'>
                  {t('shell:communityIndex.previewNotice')}
                </p>
                <div className='mt-3 flex flex-wrap items-center justify-between gap-2 text-xs'>
                  <span className='break-all font-mono'>{entry.author_pubkey} · {entry.object_id}</span>
                  <Button variant='secondary' type='button' onClick={() => setReportEntry(entry)}>
                    <ShieldAlert className='size-4' aria-hidden='true' />
                    {t('shell:report.actionLabel')}
                  </Button>
                </div>
              </article>
            </li>
          ))}
        </ul>
      ) : null}

      {reportEntry ? (
        <ReportRoutingDialog
          open={true}
          onOpenChange={(open) => { if (!open) setReportEntry(null); }}
          subject={{ kind: 'search_result', id: reportEntry.object_id, label: reportEntry.scope_id }}
          plan={reportPlan}
          onCopyContact={(value) => void copyTextToClipboard(value)}
          onSubmit={async ({ candidate, reason, details, reporterContact }) => {
            if (candidate.contact.kind !== 'endpoint') {
              throw new Error('report endpoint is unavailable');
            }
            return api.submitCommunityNodeReport({
              node_base_url: candidate.target.nodeBaseUrl,
              report_endpoint: candidate.contact.value,
              subject_kind: 'search_result',
              subject_id: reportEntry.object_id,
              capability: candidate.target.capability,
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
