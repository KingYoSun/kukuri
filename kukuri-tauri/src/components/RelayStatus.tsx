import { useTranslation } from 'react-i18next';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { useP2PStore } from '@/stores/p2pStore';
import { useShallow } from 'zustand/react/shallow';
import { Card, CardHeader, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { AlertCircle, ChevronDown, Loader2 } from 'lucide-react';
import { formatDistanceToNow } from 'date-fns';
import { getDateFnsLocale } from '@/i18n';
import { p2pApi } from '@/lib/api/p2p';
import { errorHandler } from '@/lib/errorHandler';

export const MAINLINE_RUNBOOK_URL =
  'https://github.com/KingYoSun/kukuri/blob/main/docs/03_implementation/p2p_mainline_runbook.md';

const parseBootstrapNodeId = (node: string): string | null => {
  const normalized = node.trim();
  if (normalized === '') {
    return null;
  }
  const separatorIndex = normalized.indexOf('@');
  if (separatorIndex < 0) {
    return normalized;
  }
  if (separatorIndex === 0) {
    return null;
  }
  return normalized.slice(0, separatorIndex);
};

type BootstrapInfoState = {
  source: string;
  envLocked: boolean;
  effectiveNodes: string[];
  cliNodes: string[];
  cliUpdatedAtMs: number | null;
};

export function RelayStatus() {
  const { t } = useTranslation();
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
  const [detailsOpen, setDetailsOpen] = useState(true);
  const p2pPeers = useP2PStore((state) => state.peers);

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
      errorHandler.log(t('relayStatus.fetchBootstrapFailed'), error);
    } finally {
      setBootstrapLoading(false);
    }
  }, []);

  const refreshRelaySnapshot = useCallback(async () => {
    await Promise.all([updateRelayStatus(), refreshBootstrapInfo()]);
  }, [refreshBootstrapInfo, updateRelayStatus]);

  useEffect(() => {
    void refreshBootstrapInfo();
  }, [refreshBootstrapInfo]);

  useEffect(() => {
    if (lastRelayStatusFetchedAt === null && !isFetchingRelayStatus) {
      void refreshRelaySnapshot();
    }
  }, [lastRelayStatusFetchedAt, isFetchingRelayStatus, refreshRelaySnapshot]);

  useEffect(() => {
    if (lastRelayStatusFetchedAt === null) {
      return;
    }
    const timeout = setTimeout(() => {
      void refreshRelaySnapshot();
    }, relayStatusBackoffMs);

    return () => clearTimeout(timeout);
  }, [relayStatusBackoffMs, lastRelayStatusFetchedAt, refreshRelaySnapshot]);

  const lastUpdatedLabel = useMemo(() => {
    if (!lastRelayStatusFetchedAt) {
      return t('relayStatus.notFetched');
    }
    return formatDistanceToNow(lastRelayStatusFetchedAt, {
      addSuffix: true,
      locale: getDateFnsLocale(),
    });
  }, [lastRelayStatusFetchedAt, t]);

  const cliLastUpdatedLabel = useMemo(() => {
    if (!bootstrapInfo?.cliUpdatedAtMs) {
      return t('relayStatus.notFetched');
    }
    return formatDistanceToNow(bootstrapInfo.cliUpdatedAtMs, {
      addSuffix: true,
      locale: getDateFnsLocale(),
    });
  }, [bootstrapInfo?.cliUpdatedAtMs, t]);

  const nextRefreshLabel = useMemo(() => {
    if (relayStatusBackoffMs <= 0) {
      return '—';
    }
    if (relayStatusBackoffMs >= 600_000) {
      return t('relayStatus.about10min');
    }
    if (relayStatusBackoffMs >= 300_000) {
      return t('relayStatus.about5min');
    }
    if (relayStatusBackoffMs >= 120_000) {
      return t('relayStatus.about2min');
    }
    return t('relayStatus.about30sec');
  }, [relayStatusBackoffMs, t]);

  const effectiveNodes = bootstrapInfo?.effectiveNodes ?? [];
  const cliNodes = bootstrapInfo?.cliNodes ?? [];
  const connectedBootstrapNodes = useMemo(() => {
    const connectedPeerNodeIds = new Set(
      Array.from(p2pPeers.values())
        .filter((peer) => peer.connection_status === 'connected')
        .map((peer) => peer.node_id),
    );
    const seen = new Set<string>();
    return effectiveNodes.filter((node) => {
      const normalized = node.trim();
      if (normalized === '' || seen.has(normalized)) {
        return false;
      }
      seen.add(normalized);
      const nodeId = parseBootstrapNodeId(normalized);
      return nodeId !== null && connectedPeerNodeIds.has(nodeId);
    });
  }, [effectiveNodes, p2pPeers]);
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
    await refreshRelaySnapshot();
  };

  const handleApplyCliBootstrap = async () => {
    if (!canApplyCli) {
      return;
    }
    try {
      setApplyingCli(true);
      await p2pApi.applyCliBootstrapNodes();
      await refreshRelaySnapshot();
      errorHandler.log(t('relayStatus.applyCliSuccess'), undefined, {
        showToast: true,
        toastTitle: t('relayStatus.applyLatestListTitle'),
      });
    } catch (error) {
      errorHandler.log(t('relayStatus.applyFailed'), error, {
        showToast: true,
        toastTitle: t('relayStatus.applyFailed'),
      });
    } finally {
      setApplyingCli(false);
    }
  };

  const bootstrapSourceLabel = useMemo(() => {
    const allLabels: Record<string, string> = {
      env: t('relayStatus.sourceEnv'),
      user: t('relayStatus.sourceUser'),
      bundle: t('relayStatus.sourceBundle'),
      fallback: t('relayStatus.sourceFallback'),
      none: t('relayStatus.sourceNone'),
    };
    const key = bootstrapInfo?.source ?? 'none';
    return allLabels[key] ?? allLabels.none;
  }, [bootstrapInfo?.source, t]);

  const hasRelays = relayStatus.length > 0;

  return (
    <Collapsible open={detailsOpen} onOpenChange={setDetailsOpen}>
      <Card className="mb-4" data-testid="relay-status-card">
        <CardHeader className="pb-3">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <CollapsibleTrigger asChild>
                <Button variant="ghost" size="sm" className="h-auto px-1 text-sm font-semibold">
                  <ChevronDown
                    className={`h-4 w-4 transition-transform ${
                      detailsOpen ? 'rotate-0' : '-rotate-90'
                    }`}
                  />
                  {t('relayStatus.title')}
                </Button>
              </CollapsibleTrigger>
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
              data-testid="relay-refresh-button"
            >
              {isFetchingRelayStatus ? (
                <>
                  <Loader2 className="mr-2 h-3.5 w-3.5 animate-spin" />
                  {t('relayStatus.refreshing')}
                </>
              ) : (
                t('relayStatus.retry')
              )}
            </Button>
          </div>
          <p className="text-xs text-muted-foreground mt-1" data-testid="relay-last-updated">
            {t('relayStatus.lastUpdated')}: {lastUpdatedLabel} / {t('relayStatus.nextRefresh')}:{' '}
            {nextRefreshLabel}
          </p>
        </CardHeader>
        <CollapsibleContent>
          <CardContent className="space-y-3 text-sm">
            {relayStatusError && (
              <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 p-3 text-xs text-destructive">
                <AlertCircle className="mt-0.5 h-4 w-4" />
                <div>
                  <p>{t('relayStatus.fetchFailed')}</p>
                  <p className="text-[11px] text-destructive/80">
                    {t('relayStatus.details')}: {relayStatusError}
                  </p>
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
                      ? {
                          badgeClass:
                            'bg-green-100 text-green-800 dark:bg-green-900/20 dark:text-green-300',
                          label: t('relayStatus.connected'),
                        }
                      : status === 'connecting'
                        ? {
                            badgeClass:
                              'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-300',
                            label: t('relayStatus.connecting'),
                          }
                        : status === 'disconnected'
                          ? {
                              badgeClass:
                                'bg-gray-100 text-gray-800 dark:bg-gray-800 dark:text-gray-200',
                              label: t('relayStatus.disconnected'),
                            }
                          : {
                              badgeClass:
                                'bg-red-100 text-red-800 dark:bg-red-900/20 dark:text-red-300',
                              label: t('relayStatus.error'),
                            };

                  return (
                    <div
                      key={relay.url}
                      className="flex items-center justify-between text-sm"
                      data-testid="relay-status-item"
                      data-relay-url={relay.url}
                      data-relay-status={status}
                    >
                      <span className="truncate max-w-[200px]" title={relay.url}>
                        {relay.url}
                      </span>
                      <span className={`px-2 py-1 rounded text-xs ${badgeClass}`}>{label}</span>
                    </div>
                  );
                })}
              </div>
            ) : (
              <p className="text-xs text-muted-foreground" data-testid="relay-status-empty">
                {t('relayStatus.noRelays')}
              </p>
            )}
            <div
              className="rounded-md border border-muted p-3 text-xs space-y-2 bg-muted/20"
              data-testid="relay-bootstrap-panel"
            >
              <p className="font-medium text-foreground" data-testid="relay-bootstrap-category">
                {t('relayStatus.bootstrapCategory')}
              </p>
              <div className="flex flex-wrap items-center justify-between gap-2 text-muted-foreground">
                <div className="flex flex-col">
                  <span data-testid="relay-bootstrap-source">
                    {t('relayStatus.bootstrapSource')}: {bootstrapSourceLabel}
                  </span>
                  <span
                    className="text-[11px]"
                    data-testid="relay-effective-count"
                    data-count={effectiveNodes.length}
                  >
                    {t('relayStatus.applied')}:{' '}
                    {effectiveNodes.length > 0
                      ? effectiveNodes.length
                      : t('relayStatus.sourceNone')}
                  </span>
                  <span
                    className="text-[11px]"
                    data-testid="relay-bootstrap-connected-count"
                    data-count={connectedBootstrapNodes.length}
                  >
                    {t('relayStatus.connectedBootstrapCount', {
                      count: connectedBootstrapNodes.length,
                    })}
                  </span>
                </div>
                <Button
                  variant="secondary"
                  size="sm"
                  disabled={!canApplyCli}
                  onClick={handleApplyCliBootstrap}
                  data-testid="relay-apply-cli-button"
                >
                  {applyingCli ? (
                    <>
                      <Loader2 className="mr-1 h-3.5 w-3.5 animate-spin" />
                      {t('relayStatus.switching')}
                    </>
                  ) : (
                    t('relayStatus.applyLatestList')
                  )}
                </Button>
              </div>
              <div
                className="text-muted-foreground"
                data-testid="relay-cli-info"
                data-cli-count={cliNodes.length}
                data-cli-updated-ms={bootstrapInfo?.cliUpdatedAtMs ?? null}
              >
                {t('relayStatus.cliProvided')}:{' '}
                {cliAvailable
                  ? t('relayStatus.cliInfo', { count: cliNodes.length, time: cliLastUpdatedLabel })
                  : t('relayStatus.notFetched')}
              </div>
              {envLocked && (
                <p className="text-[11px] text-muted-foreground" data-testid="relay-cli-locked">
                  <code className="font-mono text-[11px]">KUKURI_BOOTSTRAP_PEERS</code>{' '}
                  {t('relayStatus.cliLocked')}
                </p>
              )}
              <div className="space-y-1" data-testid="relay-bootstrap-connected-section">
                <p className="text-[11px] text-muted-foreground">
                  {t('relayStatus.connectedNodesTitle')}
                </p>
                {connectedBootstrapNodes.length > 0 ? (
                  <div className="space-y-1" data-testid="relay-bootstrap-connected-list">
                    {connectedBootstrapNodes.map((node) => (
                      <code
                        key={node}
                        className="block rounded bg-background/70 px-2 py-1 font-mono text-[11px] break-all"
                        data-testid="relay-bootstrap-connected-node"
                      >
                        {node}
                      </code>
                    ))}
                  </div>
                ) : (
                  <p
                    className="text-[11px] text-muted-foreground"
                    data-testid="relay-bootstrap-connected-empty"
                  >
                    {t('relayStatus.connectedBootstrapNone')}
                  </p>
                )}
              </div>
            </div>
          </CardContent>
        </CollapsibleContent>
      </Card>
    </Collapsible>
  );
}
