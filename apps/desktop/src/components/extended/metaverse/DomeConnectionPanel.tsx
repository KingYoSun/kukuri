import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';
import type { DomeBoundaryStateV1, DomeDirection, GameRoomView } from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import { Button } from '@/components/ui/button';
import { IconButton } from '@/components/ui/icon-button';
import { Notice } from '@/components/ui/notice';
import { Card } from '@/components/ui/card';
import type { MetaverseRoomActions } from './MetaverseRoomActions';
import { connectionScope, useDomeConnections, type DomeConnectionsState } from './useDomeConnections';
import { CONNECTION_DIRECTIONS, connectionRooms, connectionSlot, endpointIsCurrent } from './DomeConnectionModel';
import { DomeConnectionMap } from './DomeConnectionMap';

type Props = {
  actions: MetaverseRoomActions;
  room: GameRoomView | null;
  rooms: GameRoomView[];
  localAuthorPubkey: string;
  locale: SupportedLocale;
  connections?: DomeConnectionsState;
  boundaries?: Partial<Record<DomeDirection, DomeBoundaryStateV1>>;
};

export function DomeConnectionPanel(props: Props) {
  const own = useDomeConnections(props.actions, props.connections ? null : props.room, props.localAuthorPubkey);
  if (!props.room?.metaverse) return null;
  return <ConnectionControls key={connectionScope(props.room, props.localAuthorPubkey)} {...props}
    room={props.room} connections={props.connections ?? own} />;
}

function ConnectionControls({ actions, room, rooms, localAuthorPubkey, locale, connections, boundaries }: Props & { room: GameRoomView; connections: DomeConnectionsState }) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [direction, setDirection] = useState<DomeDirection>('north');
  const [targets, setTargets] = useState<Partial<Record<DomeDirection, string>>>({});
  const [pending, setPending] = useState(false);
  const locked = useRef(false);
  const mounted = useRef(true);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const [error, setError] = useState<string | null>(null);
  const [retry, setRetry] = useState<{ key: string; id: string } | null>(null);
  const buttonRefs = useRef<Partial<Record<DomeDirection, HTMLButtonElement | null>>>({});
  const dome = room.metaverse!;
  const { topology, status, refresh } = connections;
  const ready = topology !== null && (status === 'ready' || status === 'refreshing');
  const owner = room.host_pubkey === localAuthorPubkey && dome.instance_status === 'active' && !dome.relationship_detach;
  const candidates = connectionRooms(room, rooms).filter(r => r.metaverse!.instance_id !== dome.instance_id);
  const slot = topology ? connectionSlot(topology, room, direction) : null;
  const target = targets[direction] ?? '';
  const candidate = candidates.find(r => r.metaverse!.instance_id === target);
  const occupied = slot?.connection && ['active', 'draining'].includes(slot.connection.record.status);
  const current = slot?.connection && [slot.connection.record.agreement.proposer, slot.connection.record.agreement.receiver]
    .some(e => endpointIsCurrent(e, room));
  const busy = pending || !ready;
  const names = new Map(connectionRooms(room, rooms).map(r => [r.metaverse!.instance_id, r.title]));
  function name(id: string) { return names.get(id) ?? t('connections.map.unknown'); }
  async function run(action: () => Promise<unknown>) {
    if (locked.current || !owner || !ready) return;
    locked.current = true; setPending(true); setError(null);
    try { await action(); if (mounted.current) { setRetry(null); await refresh(); } }
    catch (failure) { if (mounted.current) setError(failure instanceof Error ? failure.message : t('connections.errors.action')); }
    finally { locked.current = false; if (mounted.current) setPending(false); }
  }
  function propose() {
    if (!candidate || occupied || !topology || slot?.status === 'unknown') return;
    const key = JSON.stringify([direction, target, candidate.metaverse!.instance_generation]);
    const id = retry?.key === key ? retry.id : `dome-proposal-${globalThis.crypto.randomUUID()}`;
    setRetry({ key, id });
    void run(() => actions.createConnectionProposal(id, dome.spatial_context, dome.instance_id, target, direction));
  }
  const boundary = slot?.status === 'draining' || slot?.status === 'blocked' || slot?.status === 'closed'
    ? slot.status : slot?.status === 'active' ? boundaries?.[direction] ?? 'loading' : undefined;
  return <Card className='shell-workspace-card metaverse-connections-card' data-metaverse-ui>
    <div className='panel-header'><h3>{t('connections.title')}</h3>
      <IconButton variant='ghost' type='button' label={t('connections.refresh')} disabled={status === 'loading' || status === 'refreshing'} onClick={() => void refresh()}>
        <RefreshCw className='size-4' aria-hidden='true' />
      </IconButton>
    </div>
    <p className='connection-scope-note'>{t('connections.map.scope')}</p>
    {(status !== 'ready' || error) && <div role='status' aria-live='polite'>
      {status === 'loading' || status === 'refreshing' ? t(`connections.map.${status}`) : null}
      {connections.error && <Notice tone='destructive'><span>{connections.error}</span>{topology ? ` — ${t('connections.map.stale')}` : ''}</Notice>}
      {error && <Notice tone='destructive'>{error}</Notice>}
    </div>}
    <div className='connection-workspace'>
      <div>
        <div className='connection-compass' role='group' aria-label={t('connections.map.directions')}>
          <span className='connection-compass-center'>{t('connections.map.here')}<strong>{room.title}</strong></span>
          {CONNECTION_DIRECTIONS.map(value => <Button key={value} type='button' variant={value === direction ? 'primary' : 'secondary'}
            className={`connection-direction connection-direction-${value}`} ref={el => { buttonRefs.current[value] = el; }}
            aria-pressed={value === direction} onClick={() => setDirection(value)}
            onKeyDown={event => {
              const next = ({ ArrowUp: 'north', ArrowRight: 'east', ArrowDown: 'south', ArrowLeft: 'west', Home: 'north', End: 'west' } as Record<string, DomeDirection>)[event.key];
              if (!next || event.nativeEvent.isComposing) return;
              event.preventDefault(); event.stopPropagation(); setDirection(next); buttonRefs.current[next]?.focus();
            }}>
            {t(`connections.directions.${value}`)}
            <small>{topology ? t(`connections.status.${connectionSlot(topology, room, value).status}`) : t('connections.map.unknown')}</small>
          </Button>)}
        </div>
        {topology && <DomeConnectionMap topology={topology} room={room} rooms={rooms} locale={locale} />}
      </div>
      <section className='connection-detail' aria-label={t('connections.map.detail', { direction: t(`connections.directions.${direction}`) })}>
        <h4>{t(`connections.directions.${direction}`)}</h4>
        {slot && <p>{t(`connections.status.${slot.status}`)}</p>}
        {boundary && <p>{t('connections.map.passage')}: {t(`connections.passage.${boundary}`)}</p>}
        {slot?.connection && <>
          <p>{t('connections.connectedTo', { name: name(slot.connection.record.agreement.proposer.instance_id === dome.instance_id
            ? slot.connection.record.agreement.receiver.instance_id : slot.connection.record.agreement.proposer.instance_id) })}</p>
          {slot.connection.record.lifecycle_reason && <p>{t(`connections.reasons.${slot.connection.record.lifecycle_reason}`)}</p>}
          {slot.connection.record.lifecycle_deadline_at && <p>{t('connections.map.drainUntil', { time: new Date(slot.connection.record.lifecycle_deadline_at).toLocaleTimeString(locale) })}</p>}
          {owner && current && slot.connection.record.status !== 'revoked' && <Button type='button' variant='secondary' disabled={busy} onClick={() => void run(() => actions.revokeConnection(dome.spatial_context, slot.connection!.record.agreement.connection_id))}>{t('connections.revoke')}</Button>}
        </>}
        {!owner && <p>{t('connections.map.ownerOnly')}</p>}
        {owner && topology && !occupied && (candidates.length ? <div className='composer composer-compact'>
          <label><span>{t('connections.target')}</span><select className='input' value={candidate ? target : ''} disabled={busy}
            onChange={e => { setTargets(values => ({ ...values, [direction]: e.target.value })); setRetry(null); }}>
            <option value=''>{t('connections.chooseTarget')}</option>
            {candidates.map(r => <option key={r.metaverse!.instance_id} value={r.metaverse!.instance_id}>{r.title}</option>)}
          </select></label>
          <Button type='button' disabled={busy || !candidate || slot?.status === 'unknown'} onClick={propose}>{t('connections.propose')}</Button>
        </div> : ready ? <p>{t('connections.map.noCandidates')}</p> : null)}
        {slot?.proposals.map(proposal => {
          const incoming = endpointIsCurrent(proposal.proposal.receiver, room);
          const outgoing = endpointIsCurrent(proposal.proposal.proposer, room);
          const actionable = !['accepted', 'discarded'].includes(proposal.status);
          return <div className='connection-proposal' key={proposal.proposal.proposal_id}>
            <strong>{name(incoming ? proposal.proposal.proposer.instance_id : proposal.proposal.receiver.instance_id)}</strong>
            <span>{t(`connections.status.${proposal.status}`)}</span>
            {proposal.terminal_reason && <span>{t(`connections.reasons.${proposal.terminal_reason}`)}</span>}
            {owner && actionable && incoming && <Button type='button' disabled={busy} onClick={() => void run(() => actions.acceptConnectionProposal(dome.spatial_context, proposal.proposal.proposal_id))}>{t('connections.accept')}</Button>}
            {owner && actionable && outgoing && <Button type='button' variant='secondary' disabled={busy} onClick={() => void run(() => actions.withdrawConnectionProposal(dome.spatial_context, proposal.proposal.proposal_id))}>{t('connections.withdraw')}</Button>}
          </div>;
        })}
      </section>
    </div>
  </Card>;
}
