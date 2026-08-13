import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import type {
  DesktopApi,
  RelationNeighborsResponse,
  RelationReadResponse,
  TrustUserReadResponse,
} from '@/lib/api';
import {
  trustRelationErrorMessage,
  trustRelationUnavailableReason,
  type TrustRelationUnavailableReason,
} from '@/lib/api/trustRelationPresentation';

type ReadState<T> =
  | { status: 'idle' | 'loading'; value: null; reason: null; message: null }
  | { status: 'ready'; value: T; reason: null; message: null }
  | { status: 'unavailable'; value: null; reason: TrustRelationUnavailableReason; message: string };

const IDLE_STATE = { status: 'idle', value: null, reason: null, message: null } as const;
const LOADING_STATE = { status: 'loading', value: null, reason: null, message: null } as const;

function resultState<T>(result: PromiseSettledResult<T>): ReadState<T> {
  if (result.status === 'fulfilled') {
    return { status: 'ready', value: result.value, reason: null, message: null };
  }
  return {
    status: 'unavailable',
    value: null,
    reason: trustRelationUnavailableReason(result.reason),
    message: trustRelationErrorMessage(result.reason),
  };
}

function continuous(value: number): string {
  return Number.isFinite(value) ? value.toFixed(3) : String(value);
}

export type CommunityNodeAdvisoryPanelProps = {
  api: Pick<
    DesktopApi,
    | 'readCommunityNodeTrustUser'
    | 'readCommunityNodeRelationUser'
    | 'listCommunityNodeRelationNeighbors'
  >;
  targetPubkey: string;
  nodeBaseUrls: string[];
};

export function CommunityNodeAdvisoryPanel({
  api,
  targetPubkey,
  nodeBaseUrls,
}: CommunityNodeAdvisoryPanelProps) {
  const { t } = useTranslation(['profile', 'common']);
  const [baseUrl, setBaseUrl] = useState(nodeBaseUrls[0] ?? '');
  const [trust, setTrust] = useState<ReadState<TrustUserReadResponse>>(IDLE_STATE);
  const [relation, setRelation] = useState<ReadState<RelationReadResponse>>(IDLE_STATE);
  const [neighbors, setNeighbors] = useState<ReadState<RelationNeighborsResponse>>(IDLE_STATE);

  useEffect(() => {
    if (!nodeBaseUrls.includes(baseUrl)) setBaseUrl(nodeBaseUrls[0] ?? '');
  }, [baseUrl, nodeBaseUrls]);

  function resetReads() {
    setTrust(IDLE_STATE);
    setRelation(IDLE_STATE);
    setNeighbors(IDLE_STATE);
  }

  async function loadAdvisory() {
    if (!baseUrl) return;
    setTrust(LOADING_STATE);
    setRelation(LOADING_STATE);
    setNeighbors(LOADING_STATE);
    const request = { base_url: baseUrl, target_pubkey: targetPubkey };
    const [trustResult, relationResult, neighborsResult] = await Promise.allSettled([
      api.readCommunityNodeTrustUser(request),
      api.readCommunityNodeRelationUser(request),
      api.listCommunityNodeRelationNeighbors({ base_url: baseUrl, limit: 20 }),
    ]);
    setTrust(resultState(trustResult));
    setRelation(resultState(relationResult));
    setNeighbors(resultState(neighborsResult));
  }

  function unavailableCopy(state: Extract<ReadState<unknown>, { status: 'unavailable' }>) {
    if (state.reason === 'other') return state.message;
    return t(`profile:communityNodeAdvisory.errors.${state.reason}`);
  }

  return (
    <section className='w-full min-w-0 max-w-full space-y-4 overflow-hidden border-t border-[var(--border-subtle)] pt-4'>
      <div className='space-y-1'>
        <h3 className='text-base font-semibold'>{t('profile:communityNodeAdvisory.title')}</h3>
        <p className='text-sm text-[var(--muted-foreground)]'>
          {t('profile:communityNodeAdvisory.nodeLocalNotice')}
        </p>
      </div>

      {nodeBaseUrls.length === 0 ? (
        <Notice>{t('profile:communityNodeAdvisory.noNodes')}</Notice>
      ) : (
        <div className='flex min-w-0 flex-wrap items-end gap-3'>
          <label className='min-w-0 flex-1 space-y-1 text-sm font-medium'>
            <span>{t('profile:communityNodeAdvisory.nodeLabel')}</span>
            <select
              className='input min-h-10 w-full min-w-0 max-w-full'
              value={baseUrl}
              onChange={(event) => {
                setBaseUrl(event.currentTarget.value);
                resetReads();
              }}
            >
              {nodeBaseUrls.map((node) => (
                <option key={node} value={node}>
                  {node}
                </option>
              ))}
            </select>
          </label>
          <Button type='button' onClick={() => void loadAdvisory()} disabled={trust.status === 'loading'}>
            {trust.status === 'loading'
              ? t('profile:communityNodeAdvisory.loading')
              : t('profile:communityNodeAdvisory.load')}
          </Button>
        </div>
      )}

      {trust.status === 'ready' ? (
        <section className='w-full min-w-0 max-w-full space-y-3 overflow-hidden rounded-[16px] bg-[var(--surface-panel-soft)] p-4'>
          <div>
            <h4 className='font-semibold'>{t('profile:communityNodeAdvisory.trust.title')}</h4>
            <p className='text-sm text-[var(--muted-foreground)]'>
              {t('profile:communityNodeAdvisory.trust.continuousNotice')}
            </p>
          </div>
          <dl
            className='grid gap-2 text-sm'
            style={{ gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' }}
          >
            <div><dt>{t('profile:communityNodeAdvisory.trust.trust')}</dt><dd>{continuous(trust.value.trust)}</dd></div>
            <div><dt>{t('profile:communityNodeAdvisory.trust.absolute')}</dt><dd>{continuous(trust.value.absolute)}</dd></div>
            <div><dt>{t('profile:communityNodeAdvisory.trust.relative')}</dt><dd>{continuous(trust.value.relative)}</dd></div>
            <div><dt>{t('profile:communityNodeAdvisory.trust.weight')}</dt><dd>{continuous(trust.value.w_abs_applied)}</dd></div>
          </dl>
          {trust.value.basis.length === 0 ? (
            <Notice>{t('profile:communityNodeAdvisory.trust.noBasis')}</Notice>
          ) : (
            <div className='space-y-2'>
              {trust.value.basis.map((basis) => (
                <details key={basis.signal_id} className='rounded-xl border border-[var(--border-subtle)] p-3 text-sm'>
                  <summary className='cursor-pointer font-medium'>
                    {basis.issuer_node_id} · {basis.category} · {basis.severity}
                  </summary>
                  <dl className='mt-3 space-y-1 break-all text-[var(--muted-foreground)]'>
                    <div><dt className='inline font-medium'>basis: </dt><dd className='inline'>{basis.basis}</dd></div>
                    <div><dt className='inline font-medium'>confidence: </dt><dd className='inline'>{basis.confidence == null ? '—' : continuous(basis.confidence)}</dd></div>
                    <div><dt className='inline font-medium'>visibility: </dt><dd className='inline'>{basis.visibility}</dd></div>
                    <div><dt className='inline font-medium'>appeal_status: </dt><dd className='inline'>{basis.appeal_status}</dd></div>
                    <div><dt className='inline font-medium'>expires_at: </dt><dd className='inline'>{basis.expires_at ?? '—'}</dd></div>
                    <div><dt className='inline font-medium'>decay_factor: </dt><dd className='inline'>{continuous(basis.decay_factor)}</dd></div>
                    <div><dt className='inline font-medium'>relation_weight: </dt><dd className='inline'>{continuous(basis.relation_weight)}</dd></div>
                    <div><dt className='inline font-medium'>contribution: </dt><dd className='inline'>{continuous(basis.contribution)}</dd></div>
                  </dl>
                </details>
              ))}
            </div>
          )}
        </section>
      ) : trust.status === 'unavailable' ? (
        <Notice>{unavailableCopy(trust)}</Notice>
      ) : null}

      {relation.status === 'ready' ? (
        <section className='w-full min-w-0 max-w-full space-y-2 overflow-hidden rounded-[16px] bg-[var(--surface-panel-soft)] p-4'>
          <h4 className='font-semibold'>{t('profile:communityNodeAdvisory.relation.title')}</h4>
          <p className='text-sm'>{t('profile:communityNodeAdvisory.relation.score', { score: continuous(relation.value.score) })}</p>
          <ul className='space-y-1 text-sm text-[var(--muted-foreground)]'>
            {relation.value.basis.map((basis) => (
              <li key={basis.feature} className='break-words'>{basis.feature}: {continuous(basis.value)} × {continuous(basis.weight)} = {continuous(basis.contribution)}</li>
            ))}
          </ul>
        </section>
      ) : relation.status === 'unavailable' ? (
        <Notice>{unavailableCopy(relation)}</Notice>
      ) : null}

      {neighbors.status === 'ready' ? (
        <section className='w-full min-w-0 max-w-full space-y-2 overflow-hidden rounded-[16px] bg-[var(--surface-panel-soft)] p-4'>
          <h4 className='font-semibold'>{t('profile:communityNodeAdvisory.neighbors.title')}</h4>
          <p className='text-sm text-[var(--muted-foreground)]'>{t('profile:communityNodeAdvisory.neighbors.notice')}</p>
          {neighbors.value.neighbors.length === 0 ? (
            <p className='text-sm'>{t('profile:communityNodeAdvisory.neighbors.empty')}</p>
          ) : (
            <ul className='space-y-1 font-mono text-xs'>
              {neighbors.value.neighbors.map((pubkey) => <li key={pubkey} className='break-all'>{pubkey}</li>)}
            </ul>
          )}
        </section>
      ) : neighbors.status === 'unavailable' ? (
        <Notice>{unavailableCopy(neighbors)}</Notice>
      ) : null}
    </section>
  );
}
