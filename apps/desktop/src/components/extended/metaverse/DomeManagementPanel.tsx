import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { GameRoomView, DomeHostingView } from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { Dialog, DialogContent, DialogTitle, DialogDescription } from '@/components/ui/dialog';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

type Props = {
  room: GameRoomView;
  actions: MetaverseRoomActions;
  endpointId: string;
  locale: SupportedLocale;
  admitted: boolean;
  onEnter: (roomId: string) => Promise<boolean>;
  onStopped: () => void;
  onDeleted: () => void;
  onClose: () => void;
};

export function DomeManagementPanel({ room, actions, endpointId, locale, admitted, onEnter, onStopped, onDeleted, onClose }: Props) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [hosting, setHosting] = useState<DomeHostingView | null>(null);
  const [pending, setPending] = useState<'start' | 'stop' | 'delete' | 'read' | 'enter' | null>('read');
  const [error, setError] = useState<string | null>(null);
  const [confirm, setConfirm] = useState(false);
  const [cleanupPending, setCleanupPending] = useState(false);
  const surface = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const frame = requestAnimationFrame(() => { surface.current?.scrollIntoView?.({ block: 'start', inline: 'nearest' }); surface.current?.focus({ preventScroll: true }); });
    return () => cancelAnimationFrame(frame);
  }, []);
  const alive = useRef(true);
  const busy = useRef(false);
  const request = useRef(0);
  const operationId = useRef(`delete-${room.metaverse!.instance_id}-${room.metaverse!.instance_generation}`);
  const dome = room.metaverse!;
  const active = hosting?.state.kind === 'owner_hosted' || hosting?.state.kind === 'community_node_hosted';
  useEffect(() => {
    alive.current = true;
    const sequence = ++request.current;
    void actions.getHosting(dome.spatial_context, dome.instance_id).then((value) => {
      if (alive.current && sequence === request.current) { setHosting(value); setPending(null); }
    }).catch(() => {
      if (alive.current && sequence === request.current) { setError(t('management.readFailed')); setPending(null); }
    });
    return () => { alive.current = false; request.current = sequence + 1; };
  }, [actions, dome.instance_id, dome.spatial_context, t]);

  async function run(kind: NonNullable<typeof pending>, action: () => Promise<void>) {
    if (busy.current) return;
    busy.current = true;
    ++request.current;
    setPending(kind); setError(null);
    try { await action(); }
    catch (cause) { if (alive.current) setError(cause instanceof Error ? cause.message : t('hosting.error')); }
    finally { busy.current = false; if (alive.current) setPending(null); }
  }
  async function enter() {
    if (!active) {
      const result = await actions.startOwnerHosting(dome.spatial_context, dome.instance_id, endpointId);
      if (!alive.current) return;
      setHosting(result);
      if (result.state.kind !== 'owner_hosted') throw new Error(t('management.startFailed'));
      await actions.refresh();
    }
    if (!alive.current) return;
    setPending('enter');
    if (!await onEnter(room.room_id) && alive.current) setError(t('management.entryFailed'));
  }
  async function remove() {
    const result = await actions.deleteRoom(dome.spatial_context, dome.instance_id, dome.instance_generation, operationId.current);
    if (!alive.current) return;
    setConfirm(false);
    if (result.cleanup_pending) { setCleanupPending(true); setError(t('management.cleanupPending')); }
    else onDeleted();
    await actions.refresh();
  }
  return <div ref={surface} tabIndex={-1}><Card className='panel-subsection' aria-label={t('management.open')} aria-busy={pending !== null}>
    <h3>{room.title} — {t('management.open')}</h3>
    <p role='status'>{pending === 'start' ? t('management.starting') : pending === 'enter' ? t('entry.admitting')
      : admitted ? t('management.entered') : hosting ? t(`hosting.states.${hosting.state.kind}`) : t('management.checking')}</p>
    <p>{t('management.explanation')}</p>
    {error ? <Notice tone='destructive'>{error}</Notice> : null}
    {cleanupPending ? <Button disabled={pending !== null} onClick={() => void run('delete', remove)}>{t('management.retryDelete')}</Button> : <>
      {!admitted ? <Button disabled={pending !== null || (!active && !endpointId) || !hosting}
        onClick={() => void run(active ? 'enter' : 'start', enter)}>{t(active ? 'room.join' : 'management.startAndEnter')}</Button> : null}
      {hosting?.lease || active || admitted ? <Button variant='secondary' disabled={pending !== null} onClick={() => void run('stop', async () => {
        const result = await actions.closeHosting(dome.spatial_context, dome.instance_id);
        if (!alive.current) return;
        setHosting(result); onStopped(); await actions.refresh();
      })}>{t('hosting.close')}</Button> : null}
      <Button variant='secondary' disabled={pending !== null} onClick={() => void run('read', async () => {
        const value = await actions.getHosting(dome.spatial_context, dome.instance_id);
        if (alive.current) setHosting(value);
      })}>{t('management.refresh')}</Button>
      <Button variant='secondary' className='text-destructive' disabled={pending !== null} onClick={() => setConfirm(true)}>{t('management.delete')}</Button>
    </>}
    <Button variant='secondary' disabled={pending !== null} onClick={onClose}>{t('management.close')}</Button>
    <Dialog open={confirm} onOpenChange={(open) => { if (pending !== 'delete') setConfirm(open); }}>
      <DialogContent hideClose={pending === 'delete'}>
        <DialogTitle>{t('management.deleteTitle', { title: room.title })}</DialogTitle>
        <DialogDescription>{t('management.deleteDescription')}</DialogDescription>
        {error ? <Notice tone='destructive'>{error}</Notice> : null}
        <Button variant='secondary' disabled={pending === 'delete'} onClick={() => setConfirm(false)}>{t('management.cancel')}</Button>
        <Button variant='secondary' className='text-destructive' disabled={pending === 'delete'} onClick={() => void run('delete', remove)}>{t('management.confirmDelete')}</Button>
      </DialogContent>
    </Dialog>
  </Card></div>;
}
