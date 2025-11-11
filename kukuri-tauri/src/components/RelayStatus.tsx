import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useShallow } from 'zustand/react/shallow';
import { Card, CardHeader, CardTitle, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { AlertCircle, Loader2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { ja } from 'date-fns/locale';
import { p2pApi } from '@/lib/api/p2p';
import { errorHandler } from '@/lib/errorHandler';

export const MAINLINE_RUNBOOK_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/03_implementation/p2p_mainline_runbook.md';

type BootstrapInfoState = {
  source: string;
  envLocked: boolean;
  effectiveNodes: string[];
  cliNodes: string[];
  cliUpdatedAtMs: number | null;
};

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
  const [bootstrapInfo, setBootstrapInfo] = useState<BootstrapInfoState | null>(null);
  const [bootstrapLoading, setBootstrapLoading] = useState(false);
  const [applyingCli, setApplyingCli] = useState(false);

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

  const refreshBootstrapInfo = useCallback(async () => {
    try {
      setBootstrapLoading(true);
      const config = await p2pApi.getBootstrapConfig();
      setBootstrapInfo({
        source: config.source ?? 'none',
        envLocked: Boolean(config.env_locked),
        effectiveNodes: config.effective_nodes ?? [],
        cliNodes: config.cli_nodes ?? [],
        cliUpdatedAtMs: config.cli_updated_at_ms ?? null,
      });
    } catch (error) {
      errorHandler.log('ブートストラップ設定の取得に失敗しました', error);
    } finally {
      setBootstrapLoading(false);
    }
  }, []);

  useEffect(() => {
    void refreshBootstrapInfo();
  }, [refreshBootstrapInfo]);

  const lastUpdatedLabel = useMemo(() => {
    if (!lastRelayStatusFetchedAt) {
      return '未取得';
    }
    return formatDistanceToNow(lastRelayStatusFetchedAt, {
      addSuffix: true,
      locale: ja,
    });
  }, [lastRelayStatusFetchedAt]);

  const cliLastUpdatedLabel = useMemo(() => {
    if (!bootstrapInfo?.cliUpdatedAtMs) {
      return '未取得';
    }
    return formatDistanceToNow(bootstrapInfo.cliUpdatedAtMs, {
      addSuffix: true,
      locale: ja,
    });
  }, [bootstrapInfo?.cliUpdatedAtMs]);

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

  const effectiveNodes = bootstrapInfo?.effectiveNodes ?? [];
  const cliNodes = bootstrapInfo?.cliNodes ?? [];
  const cliAvailable = cliNodes.length > 0;
  const envLocked = bootstrapInfo?.envLocked ?? false;
  const normalized = (values: string[]) => [...values].sort().join('|');
  const cliMatchesEffective = cliAvailable && normalized(cliNodes) === normalized(effectiveNodes);
  const canApplyCli =
    cliAvailable && !envLocked && !cliMatchesEffective && !applyingCli && !bootstrapLoading;

  const handleManualRefresh = async () => {
    if (isFetchingRelayStatus) {
      return;
    }
    await updateRelayStatus();
  };

  const handleApplyCliBootstrap = async () => {
    if (!canApplyCli) {
      return;
    }
    try {
      setApplyingCli(true);
      await p2pApi.applyCliBootstrapNodes();
      await Promise.all([updateRelayStatus(), refreshBootstrapInfo()]);
      errorHandler.log('CLIブートストラップリストを適用しました', undefined, {
        showToast: true,
        toastTitle: '最新リストを適用',
      });
    } catch (error) {
      errorHandler.log('最新リストの適用に失敗しました', error, {
        showToast: true,
        toastTitle: '適用に失敗しました',
      });
    } finally {
      setApplyingCli(false);
    }
  };

  const bootstrapSourceLabel = useMemo(() => {
    const allLabels: Record<string, string> = {
      env: '環境変数 (KUKURI_BOOTSTRAP_PEERS)',
      user: 'ユーザー設定',
      bundle: '同梱設定ファイル',
      fallback: 'フォールバック',
      none: 'n0 デフォルト',
    };
    const key = bootstrapInfo?.source ?? 'none';
    return allLabels[key] ?? allLabels.none;
  }, [bootstrapInfo?.source]);

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
        <div className="rounded-md border border-muted p-3 text-xs space-y-2 bg-muted/20">
          <div className="flex flex-wrap items-center justify-between gap-2 text-muted-foreground">
            <div className="flex flex-col">
              <span>ブートストラップソース: {bootstrapSourceLabel}</span>
              <span className="text-[11px]">
                適用中: {effectiveNodes.length > 0 ? effectiveNodes.length : 'n0 デフォルト'}
              </span>
            </div>
            <Button
              variant="secondary"
              size="sm"
              disabled={!canApplyCli}
              onClick={handleApplyCliBootstrap}
            >
              {applyingCli ? (
                <>
                  <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
                  切替中…
                </>
              ) : (
                '最新リストを適用'
              )}
            </Button>
          </div>
          <div className="text-muted-foreground">
            CLI 提供:{' '}
            {cliAvailable ? `${cliNodes.length}件 / 更新: ${cliLastUpdatedLabel}` : '未取得'}
          </div>
          {envLocked && (
            <p className="text-[11px] text-muted-foreground">
              <code className="font-mono text-[11px]">KUKURI_BOOTSTRAP_PEERS</code>{' '}
              が設定されているため CLIリストを適用できません。
            </p>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
