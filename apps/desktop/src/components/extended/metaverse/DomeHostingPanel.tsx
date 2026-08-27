import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { SupportedLocale } from '@/i18n';
import type { DomeHostingView, GameRoomView } from '@/lib/api';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

type DomeHostingPanelProps = {
  actions: MetaverseRoomActions;
  room: GameRoomView | null;
  localAuthorPubkey: string;
  localEndpointId: string;
  locale: SupportedLocale;
};

export function DomeHostingPanel({
  actions,
  room,
  localAuthorPubkey,
  localEndpointId,
  locale,
}: DomeHostingPanelProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [nodeId, setNodeId] = useState('');
  const [baseUrl, setBaseUrl] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
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
      </div>
      {error ? <Notice tone='destructive'>{error}</Notice> : null}
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
        </div>
      ) : null}
    </Card>
  );
}
