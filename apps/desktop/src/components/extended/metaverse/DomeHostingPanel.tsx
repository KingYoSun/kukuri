import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { SupportedLocale } from '@/i18n';
import type { DomeHostingView, GameRoomView } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { InvokeError } from '@/lib/api/invoke/error';
import { formatBytes } from '@/shell/presentation';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

type DomeHostingPanelProps = {
  actions: MetaverseRoomActions;
  room: GameRoomView | null;
  localAuthorPubkey: string;
  localEndpointId: string;
  locale: SupportedLocale;
  onSpawnGuestProp: () => Promise<void>;
  onAddPersistentProp: () => Promise<void>;
  onDeletePersistentProp: () => Promise<void>;
};

export function DomeHostingPanel({
  actions,
  room,
  localAuthorPubkey,
  localEndpointId,
  locale,
  onSpawnGuestProp,
  onAddPersistentProp,
  onDeletePersistentProp,
}: DomeHostingPanelProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [nodeId, setNodeId] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [layoutResult, setLayoutResult] = useState<string | null>(null);
  const [resyncResult, setResyncResult] = useState<string | null>(null);
  const [hosting, setHosting] = useState<DomeHostingView | null>(null);
  useEffect(() => {
    if (!room?.metaverse) {
      setHosting(null);
      return;
    }
    void actions
      .getHosting(room.metaverse.spatial_context, room.metaverse.instance_id)
      .then(setHosting)
      .catch(() => setHosting(null));
  }, [actions, room?.metaverse]);
  if (!room?.metaverse) return null;

  const isOwner = room.host_pubkey === localAuthorPubkey;
  const state = hosting?.state ?? room.dome_hosting;
  const run = async (action: () => Promise<unknown>) => {
    setPending(true);
    setError(null);
    try {
      await action();
      setHosting(await actions.getHosting(room.metaverse!.spatial_context, room.metaverse!.instance_id));
      await actions.refresh();
    } catch (cause) {
      setError(
        cause instanceof InvokeError && cause.code.startsWith('METAVERSE_')
          ? t('hosting.resourceRejected', { code: cause.code })
          : cause instanceof Error ? cause.message : t('hosting.error')
      );
    } finally {
      setPending(false);
    }
  };
  const saveLayout = async () => {
    setPending(true);
    setError(null);
    setLayoutResult(null);
    try {
      const result = await actions.commitLayout(
        room.metaverse!.spatial_context,
        room.metaverse!.instance_id,
        globalThis.crypto?.randomUUID?.() ?? `layout-${Date.now()}`
      );
      setHosting(result.hosting);
      setLayoutResult(t(`hosting.layout.${result.outcome}`, { revision: result.revision }));
      await actions.refresh();
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : t('hosting.error'));
    } finally {
      setPending(false);
    }
  };
  const resync = async () => {
    setPending(true);
    setError(null);
    try {
      const snapshots = await actions.resyncSnapshots(
        room.metaverse!.spatial_context,
        room.metaverse!.instance_id,
        0
      );
      setResyncResult(t('hosting.resyncResult', { count: snapshots.length }));
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : t('hosting.error'));
    } finally {
      setPending(false);
    }
  };

  return (
    <Card className='panel-subsection' aria-busy={pending}>
      <CardHeader>
        <h3>{t('hosting.title')}</h3>
        <small>{t(`hosting.states.${state?.kind ?? 'closed'}`)}</small>
      </CardHeader>
      <div className='topic-diagnostic topic-diagnostic-secondary'>
        <span>{t('hosting.epoch', { value: state?.lease_epoch ?? 0 })}</span>
        <span>{t('hosting.session', { value: state?.session_id ?? t('hosting.none') })}</span>
        <span>{t('hosting.expiry', { value: state?.lease_expires_at ? new Date(state.lease_expires_at).toLocaleString(locale) : t('hosting.none') })}</span>
        <span>{t('hosting.participants', { value: hosting?.participants ?? 0 })}</span>
        <span>{t(`hosting.sleep.${hosting?.sleeping === false ? 'awake' : 'sleeping'}`)}</span>
        <span>{t('hosting.revision', { value: room.metaverse.preset_ref.revision })}</span>
        <span>{t('hosting.cacheLimit', {
          value: formatBytes(hosting?.resource_budget.client.cache_capacity_bytes ?? 0, locale)
        })}</span>
        <span>{t('hosting.participantBudget', {
          used: hosting?.participants ?? 0,
          limit: hosting?.resource_budget.host.max_participants ?? 0
        })}</span>
        <span>{t('hosting.rigidBodyBudget', {
          used: hosting?.resource_metrics.rigid_body_high_water ?? 0,
          limit: hosting?.resource_budget.host.max_simulated_rigid_bodies ?? 0
        })}</span>
        <span>{t('hosting.rejectedResources', {
          value: hosting?.resource_metrics.rejected_total ?? 0
        })}</span>
        <span>{t('hosting.snapshotBytes', {
          value: formatBytes(hosting?.resource_metrics.snapshot_bytes ?? 0, locale)
        })}</span>
      </div>
      {error ? <Notice tone='destructive'>{error}</Notice> : null}
      {layoutResult ? <Notice>{layoutResult}</Notice> : null}
      {resyncResult ? <Notice>{resyncResult}</Notice> : null}
      {!isOwner ? <Notice>{t('hosting.ownerOnly')}</Notice> : null}
      {isOwner ? (
        <div className='composer composer-compact'>
          <Button
            type='button'
            disabled={pending || !localEndpointId}
            onClick={() => void run(() => actions.startOwnerHosting(
              room.metaverse!.spatial_context,
              room.metaverse!.instance_id,
              localEndpointId
            ))}
          >
            {t('hosting.ownerHost')}
          </Button>
          <Label>
            <span>{t('hosting.nodeId')}</span>
            <Input value={nodeId} disabled={pending} onChange={(event) => setNodeId(event.target.value)} />
          </Label>
          <Label>
            <span>{t('hosting.baseUrl')}</span>
            <Input value={baseUrl} disabled={pending} placeholder='https://community.example' onChange={(event) => setBaseUrl(event.target.value)} />
          </Label>
          <Button
            type='button'
            variant='secondary'
            disabled={pending || !nodeId.trim() || !baseUrl.trim()}
            onClick={() => void run(() => actions.delegateHosting(
              room.metaverse!.spatial_context,
              room.metaverse!.instance_id,
              nodeId.trim(),
              baseUrl.trim()
            ))}
          >
            {t('hosting.delegate')}
          </Button>
          <Button
            type='button'
            variant='secondary'
            disabled={pending || !state || state.kind === 'closed'}
            onClick={() => void run(() => actions.closeHosting(
              room.metaverse!.spatial_context,
              room.metaverse!.instance_id
            ))}
          >
            {t('hosting.close')}
          </Button>
          <Button
            type='button'
            disabled={pending || !state || state.kind === 'closed' || state.kind === 'transferring'}
            onClick={() => void saveLayout()}
          >
            {t('hosting.saveLayout')}
          </Button>
          <Button
            type='button'
            variant='secondary'
            disabled={pending || !state || state.kind === 'closed' || state.kind === 'transferring'}
            onClick={() => void run(onAddPersistentProp)}
          >
            {t('hosting.addPersistentProp')}
          </Button>
          <Button
            type='button'
            variant='secondary'
            disabled={pending || !state || state.kind === 'closed' || state.kind === 'transferring'}
            onClick={() => void run(onDeletePersistentProp)}
          >
            {t('hosting.deletePersistentProp')}
          </Button>
        </div>
      ) : null}
      {state && state.kind !== 'closed' ? (
        <div className='composer composer-compact'>
          <Button type='button' variant='secondary' disabled={pending} onClick={() => void run(onSpawnGuestProp)}>
            {t('hosting.spawnGuestProp')}
          </Button>
          <Button type='button' variant='secondary' disabled={pending} onClick={() => void resync()}>
            {t('hosting.resync')}
          </Button>
        </div>
      ) : null}
    </Card>
  );
}
