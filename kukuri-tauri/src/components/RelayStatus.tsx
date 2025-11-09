import { useEffect, useMemo } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useShallow } from 'zustand/react/shallow';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { AlertCircle, Loader2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';

export const MAINLINE_RUNBOOK_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/03_implementation/p2p_mainline_runbook.md';

export function RelayStatus() {
  const {
    relayStatus,
    updateRelayStatus,
    relayStatusError,
    relayStatusBackoffMs,
    lastRelayStatusFetchedAt,
    isFetchingRelayStatus,
  } = useAuthStore(
    useShallow((state) => ({
      relayStatus: state.relayStatus,
      updateRelayStatus: state.updateRelayStatus,
      relayStatusError: state.relayStatusError,
      relayStatusBackoffMs: state.relayStatusBackoffMs,
      lastRelayStatusFetchedAt: state.lastRelayStatusFetchedAt,
      isFetchingRelayStatus: state.isFetchingRelayStatus,
    })),
  );

  useEffect(() => {
    if (lastRelayStatusFetchedAt === null && !isFetchingRelayStatus) {
      void updateRelayStatus();
    }
  }, [lastRelayStatusFetchedAt, isFetchingRelayStatus, updateRelayStatus]);

  useEffect(() => {
    if (lastRelayStatusFetchedAt === null) {
      return;
    }
    const timeout = setTimeout(() => {
      void updateRelayStatus();
    }, relayStatusBackoffMs);

    return () => clearTimeout(timeout);
  }, [relayStatusBackoffMs, lastRelayStatusFetchedAt, updateRelayStatus]);

  const lastUpdatedLabel = useMemo(() => {
    if (!lastRelayStatusFetchedAt) {
      return '未取得';
    }
    return formatDistanceToNow(lastRelayStatusFetchedAt, {
      addSuffix: true,
      locale: ja,
    });
  }, [lastRelayStatusFetchedAt]);

  const nextRefreshLabel = useMemo(() => {
    if (relayStatusBackoffMs <= 0) {
      return '—';
    }
    if (relayStatusBackoffMs >= 600_000) {
      return '約10分';
    }
    if (relayStatusBackoffMs >= 300_000) {
      return '約5分';
    }
    if (relayStatusBackoffMs >= 120_000) {
      return '約2分';
    }
    return '約30秒';
  }, [relayStatusBackoffMs]);

  const handleManualRefresh = async () => {
    if (isFetchingRelayStatus) {
      return;
    }
    await updateRelayStatus();
  };

  const hasRelays = relayStatus.length > 0;

  return (
    <Card className="mb-4">
      <CardHeader className="pb-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <div className="flex items-center gap-2">
            <CardTitle className="text-sm">リレー接続状態</CardTitle>
            <Button variant="link" size="sm" className="-ml-1 h-auto px-0 text-xs" asChild>
              <a
                href={MAINLINE_RUNBOOK_URL}
                target="_blank"
                rel="noreferrer"
                data-testid="relay-runbook-link"
              >
                Runbook
              </a>
            </Button>
          </div>
          <Button
            variant="outline"
            size="sm"
            onClick={handleManualRefresh}
            disabled={isFetchingRelayStatus}
          >
            {isFetchingRelayStatus ? (
              <>
                <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                更新中…
              </>
            ) : (
              '再試行'
            )}
          </Button>
        </div>
        <p className="text-xs text-muted-foreground mt-1">
          最終更新: {lastUpdatedLabel} / 次回再取得: {nextRefreshLabel}
        </p>
      </CardHeader>
      <CardContent className="space-y-3 text-sm">
        {relayStatusError && (
          <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
            <AlertCircle className="mt-0.5 h-4 w-4" />
            <div>
              <p>リレー状態の取得に失敗しました。</p>
              <p className="text-[11px] text-destructive/80">詳細: {relayStatusError}</p>
            </div>
          </div>
        )}

        {hasRelays ? (
          <div className="space-y-2">
            {relayStatus.map((relay) => {
              const statusRaw = relay.status.toLowerCase();
              const status = statusRaw.startsWith('error') ? 'error' : statusRaw;
              const { badgeClass, label } =
                status === 'connected'
                  ? { badgeClass: 'bg-green-100 text-green-800', label: '接続済み' }
                  : status === 'connecting'
                    ? { badgeClass: 'bg-yellow-100 text-yellow-800', label: '接続中' }
                    : status === 'disconnected'
                      ? { badgeClass: 'bg-gray-100 text-gray-800', label: '切断' }
                      : { badgeClass: 'bg-red-100 text-red-800', label: 'エラー' };

              return (
                <div key={relay.url} className="flex items-center justify-between text-sm">
                  <span className="truncate max-w-[200px]" title={relay.url}>
                    {relay.url}
                  </span>
                  <span className={`px-2 py-1 rounded text-xs ${badgeClass}`}>{label}</span>
                </div>
              );
            })}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">接続中のリレーはありません。</p>
        )}
      </CardContent>
    </Card>
  );
}
