import { useCallback, useEffect, useMemo, useState } from 'react';
import { Link2, RefreshCw, Unlink } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import type { SupportedLocale } from '@/i18n';
import type {
  DomeConnectionProposalView,
  DomeConnectionTopologyView,
  DomeDirection,
  GameRoomView,
} from '@/lib/api';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

const DIRECTIONS: DomeDirection[] = ['north', 'east', 'south', 'west'];

type DomeConnectionPanelProps = {
  actions: MetaverseRoomActions;
  room: GameRoomView | null;
  rooms: GameRoomView[];
  localAuthorPubkey: string;
  locale: SupportedLocale;
};

export function DomeConnectionPanel({
  actions,
  room,
  rooms,
  localAuthorPubkey,
  locale,
}: DomeConnectionPanelProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [topology, setTopology] = useState<DomeConnectionTopologyView | null>(null);
  const [targetByDirection, setTargetByDirection] = useState<Partial<Record<DomeDirection, string>>>({});
  const [pendingAction, setPendingAction] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const context = room?.metaverse?.spatial_context ?? null;
  const roomId = room?.room_id ?? null;
  const isOwner = room?.host_pubkey === localAuthorPubkey;

  const refresh = useCallback(async () => {
    if (!context) return;
    try {
      setTopology(await actions.listConnections(context));
      setError(null);
    } catch (loadError) {
      setError(loadError instanceof Error ? loadError.message : t('connections.errors.load'));
    }
  }, [actions, context, t]);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 5_000);
    return () => window.clearInterval(interval);
  }, [refresh]);

  const candidates = useMemo(
    () =>
      rooms.filter(
        (candidate) =>
          candidate.room_id !== room?.room_id &&
          candidate.metaverse?.instance_status === 'active' &&
          candidate.metaverse.relationship_detach == null &&
          JSON.stringify(candidate.metaverse.spatial_context) === JSON.stringify(context)
      ),
    [context, room?.room_id, rooms]
  );

  if (!room?.metaverse || !context || !roomId) return null;

  async function runAction(key: string, action: () => Promise<unknown>) {
    setPendingAction(key);
    try {
      await action();
      await refresh();
      setError(null);
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : t('connections.errors.action'));
    } finally {
      setPendingAction(null);
    }
  }

  function proposalsFor(direction: DomeDirection) {
    return (topology?.proposals ?? []).filter(
      (proposal) =>
        (proposal.proposal.proposer.instance_id === roomId &&
          proposal.proposal.proposer.direction === direction) ||
        (proposal.proposal.receiver.instance_id === roomId &&
          proposal.proposal.receiver.direction === direction)
    );
  }

  function activeConnection(direction: DomeDirection) {
    return topology?.connections.find(
      ({ record }) =>
        (record.status === 'active' || record.status === 'draining') &&
        ((record.agreement.proposer.instance_id === roomId &&
          record.agreement.proposer.direction === direction) ||
          (record.agreement.receiver.instance_id === roomId &&
            record.agreement.receiver.direction === direction))
    );
  }

  function otherEndpointLabel(proposal: DomeConnectionProposalView) {
    const endpoint =
      proposal.proposal.proposer.instance_id === roomId
        ? proposal.proposal.receiver
        : proposal.proposal.proposer;
    return (
      rooms.find((candidate) => candidate.room_id === endpoint.instance_id)?.title ??
      t('common:fallbacks.unknown')
    );
  }

  return (
    <Card className='shell-workspace-card metaverse-connections-card'>
      <div className='panel-header'>
        <div>
          <h3>{t('connections.title')}</h3>
          <small>{t('connections.summary', { count: topology?.resolution.topology.components.length ?? 0 })}</small>
        </div>
        <Button variant='ghost' size='icon' type='button' aria-label={t('connections.refresh')} onClick={() => void refresh()}>
          <RefreshCw className='size-4' aria-hidden='true' />
        </Button>
      </div>
      {error ? <Notice tone='destructive'>{error}</Notice> : null}
      <div className='metaverse-connection-slots'>
        {DIRECTIONS.map((direction) => {
          const connection = activeConnection(direction);
          const proposals = proposalsFor(direction);
          const target = targetByDirection[direction] ?? '';
          return (
            <section key={direction} className='post-card' aria-labelledby={`connection-slot-${direction}`}>
              <div className='post-meta'>
                <h4 id={`connection-slot-${direction}`}>{t(`connections.directions.${direction}`)}</h4>
                <span>{connection ? t(`connections.status.${connection.record.status}`) : t('connections.status.open')}</span>
              </div>
              {connection ? (
                <div className='post-actions'>
                  <span>{t('connections.connectedTo', {
                    name:
                      rooms.find((candidate) =>
                        [
                          connection.record.agreement.proposer.instance_id,
                          connection.record.agreement.receiver.instance_id,
                        ].includes(candidate.room_id) && candidate.room_id !== room.room_id
                      )?.title ?? t('common:fallbacks.unknown'),
                  })}</span>
                  {isOwner ? (
                    <Button
                      variant='secondary'
                      type='button'
                      disabled={pendingAction !== null}
                      onClick={() => void runAction(`revoke-${direction}`, () =>
                        actions.revokeConnection(context, connection.record.agreement.connection_id)
                      )}
                    >
                      <Unlink className='size-4' aria-hidden='true' />
                      {t('connections.revoke')}
                    </Button>
                  ) : null}
                </div>
              ) : isOwner ? (
                <div className='composer composer-compact'>
                  <Label>
                    <span>{t('connections.target')}</span>
                    <select
                      className='input'
                      value={target}
                      disabled={pendingAction !== null || candidates.length === 0}
                      onChange={(event) =>
                        setTargetByDirection((current) => ({ ...current, [direction]: event.target.value }))
                      }
                    >
                      <option value=''>{t('connections.chooseTarget')}</option>
                      {candidates.map((candidate) => (
                        <option key={candidate.room_id} value={candidate.room_id}>{candidate.title}</option>
                      ))}
                    </select>
                  </Label>
                  <Button
                    type='button'
                    disabled={!target || pendingAction !== null}
                    onClick={() => {
                      const suffix = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}`;
                      void runAction(`propose-${direction}`, () =>
                        actions.createConnectionProposal(
                          `dome-proposal-${suffix}`,
                          context,
                          room.room_id,
                          target,
                          direction
                        )
                      );
                    }}
                  >
                    <Link2 className='size-4' aria-hidden='true' />
                    {t('connections.propose')}
                  </Button>
                </div>
              ) : null}
              {proposals.length > 0 ? (
                <ul className='post-list'>
                  {proposals.map((proposal) => {
                    const incoming = proposal.proposal.receiver.instance_id === room.room_id;
                    const actionable = !['accepted', 'discarded'].includes(proposal.status);
                    return (
                      <li key={proposal.proposal.proposal_id} className='post-card'>
                        <div className='post-meta'>
                          <span>{otherEndpointLabel(proposal)}</span>
                          <span>{t(`connections.status.${proposal.status}`)}</span>
                        </div>
                        {isOwner && actionable ? (
                          <div className='post-actions'>
                            {incoming ? (
                              <Button
                                type='button'
                                disabled={pendingAction !== null}
                                onClick={() => void runAction(`accept-${proposal.proposal.proposal_id}`, () =>
                                  actions.acceptConnectionProposal(context, proposal.proposal.proposal_id)
                                )}
                              >
                                {t('connections.accept')}
                              </Button>
                            ) : (
                              <Button
                                variant='secondary'
                                type='button'
                                disabled={pendingAction !== null}
                                onClick={() => void runAction(`withdraw-${proposal.proposal.proposal_id}`, () =>
                                  actions.withdrawConnectionProposal(context, proposal.proposal.proposal_id)
                                )}
                              >
                                {t('connections.withdraw')}
                              </Button>
                            )}
                          </div>
                        ) : null}
                      </li>
                    );
                  })}
                </ul>
              ) : null}
            </section>
          );
        })}
      </div>
      {topology ? (
        <p className='topic-diagnostic topic-diagnostic-secondary'>
          {t('connections.topology', {
            digest: topology.resolution.topology.topology_digest.slice(0, 12),
            count: topology.resolution.topology.active_connection_ids.length,
          })}
        </p>
      ) : null}
    </Card>
  );
}
