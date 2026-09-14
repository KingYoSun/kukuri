import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { SpatialContextV1 } from '@/lib/api';
import type { SupportedLocale } from '@/i18n';
import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import type { MetaverseRoomActions } from './MetaverseRoomActions';

export function PendingDomeDeletions({ actions, context, locale }: { actions: MetaverseRoomActions; context: SpatialContextV1; locale: SupportedLocale }) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [entries, setEntries] = useState<Awaited<ReturnType<MetaverseRoomActions['listPendingDeletions']>>>([]);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    let cancelled = false;
    void actions.listPendingDeletions(context).then((value) => { if (!cancelled) { setEntries(value); setError(null); } })
      .catch(() => { if (!cancelled) setError(t('management.pendingReadFailed')); });
    return () => { cancelled = true; };
  }, [actions, context, refresh, t]);
  return <>
    {error ? <Notice>{error}<Button variant='secondary' disabled={pending} onClick={() => setRefresh((value) => value + 1)}>{t('management.refresh')}</Button></Notice> : null}
    {entries.map((entry) => <Notice key={entry.request.operation_id}>
      <span>{entry.title} — {t('management.retryDelete')}</span>
      <Button variant='secondary' disabled={pending} onClick={async () => {
        setPending(true); setError(null);
        try {
          const request = entry.request;
          const result = await actions.deleteRoom(request.spatial_context, request.instance_id, request.expected_generation, request.operation_id);
          if (result.cleanup_pending) setError(t('management.cleanupPending'));
          setEntries(await actions.listPendingDeletions(context));
          await actions.refresh();
        } catch (cause) { setError(cause instanceof Error ? cause.message : t('hosting.error')); }
        finally { setPending(false); }
      }}>{t('management.retryDelete')}</Button>
    </Notice>)}
  </>;
}
