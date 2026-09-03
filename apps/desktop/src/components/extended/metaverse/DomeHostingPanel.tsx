import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { SupportedLocale } from '@/i18n';
import type {
  CommunityNodeConsentDocumentRef,
  DomeHostingView,
  GameRoomView,
} from '@/lib/api';
import { CommunityNodeConsentDialog } from '@/components/settings/CommunityNodeConsentDialog';
import type { CommunityNodeEntryView } from '@/components/settings/types';
import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';
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
  communityNodes?: CommunityNodeEntryView[];
  onFetchCommunityNodeConsents?: (baseUrl: string) => Promise<void>;
  onAcceptCommunityNodeConsents?: (
    baseUrl: string,
    documents: CommunityNodeConsentDocumentRef[]
  ) => Promise<void>;
  onOpenCommunityNodeSettings?: () => void;
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
  communityNodes = [],
  onFetchCommunityNodeConsents,
  onAcceptCommunityNodeConsents,
  onOpenCommunityNodeSettings,
}: DomeHostingPanelProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [selectedNodeBaseUrl, setSelectedNodeBaseUrl] = useState('');
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [layoutResult, setLayoutResult] = useState<string | null>(null);
  const [resyncResult, setResyncResult] = useState<string | null>(null);
  const [hosting, setHosting] = useState<DomeHostingView | null>(null);
  const [consentDialogNodeBaseUrl, setConsentDialogNodeBaseUrl] = useState<string | null>(null);
  const [consentDialogLoadedBaseUrl, setConsentDialogLoadedBaseUrl] = useState<string | null>(null);
  const [consentDialogFetchError, setConsentDialogFetchError] = useState<string | null>(null);
  const [consentBusy, setConsentBusy] = useState(false);
  const savedCommunityNodes = communityNodes.filter((node) => node.saved && node.baseUrl.trim());
  const selectableCommunityNodes = savedCommunityNodes.filter((node) => node.nodeId?.trim());
  const selectedCommunityNode = selectableCommunityNodes.find(
    (node) => node.baseUrl === selectedNodeBaseUrl
  );
  const consentDialogNode = communityNodes.find(
    (node) => node.baseUrl === consentDialogNodeBaseUrl
  );
  const consentDialogView = consentDialogNode
    ? {
        ...consentDialogNode.consent,
        loaded:
          consentDialogNode.consent.loaded &&
          consentDialogLoadedBaseUrl === consentDialogNode.baseUrl,
        loadError: consentDialogFetchError ?? consentDialogNode.consent.loadError,
      }
    : null;

  useEffect(() => {
    if (
      selectedNodeBaseUrl &&
      selectableCommunityNodes.some((node) => node.baseUrl === selectedNodeBaseUrl)
    ) {
      return;
    }
    setSelectedNodeBaseUrl(selectableCommunityNodes[0]?.baseUrl ?? '');
  }, [selectedNodeBaseUrl, selectableCommunityNodes]);

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
  const openConsentDialog = async (node: CommunityNodeEntryView) => {
    if (!onFetchCommunityNodeConsents || !onAcceptCommunityNodeConsents) {
      setError(t('hosting.consentUnavailable'));
      return;
    }
    setConsentDialogNodeBaseUrl(node.baseUrl);
    setConsentDialogLoadedBaseUrl(null);
    setConsentDialogFetchError(null);
    setConsentBusy(true);
    try {
      await onFetchCommunityNodeConsents(node.baseUrl);
      setConsentDialogLoadedBaseUrl(node.baseUrl);
    } catch (fetchError) {
      setConsentDialogFetchError(
        fetchError instanceof Error ? fetchError.message : String(fetchError)
      );
    } finally {
      setConsentBusy(false);
    }
  };
  const run = async (
    action: () => Promise<unknown>,
    consentTarget?: CommunityNodeEntryView
  ) => {
    setPending(true);
    setError(null);
    try {
      await action();
      setHosting(await actions.getHosting(room.metaverse!.spatial_context, room.metaverse!.instance_id));
      await actions.refresh();
    } catch (cause) {
      if (
        cause instanceof InvokeError &&
        cause.code === 'CONSENT_REQUIRED' &&
        consentTarget
      ) {
        await openConsentDialog(consentTarget);
        return;
      }
      setError(
        cause instanceof InvokeError && cause.code.startsWith('METAVERSE_')
          ? t('hosting.resourceRejected', { code: cause.code })
          : cause instanceof Error ? cause.message : t('hosting.error')
      );
    } finally {
      setPending(false);
    }
  };
  const delegateToCommunityNode = async (node: CommunityNodeEntryView) => {
    if (!node.nodeId?.trim()) return;
    await run(
      () =>
        actions.delegateHosting(
          room.metaverse!.spatial_context,
          room.metaverse!.instance_id,
          node.nodeId!.trim(),
          node.baseUrl
        ),
      node
    );
  };
  const requestCommunityNodeDelegation = async () => {
    if (!selectedCommunityNode) return;
    if (
      !selectedCommunityNode.consent.hasLocalConsent ||
      selectedCommunityNode.consent.withdrawn ||
      selectedCommunityNode.consent.hasPendingUpdate
    ) {
      await openConsentDialog(selectedCommunityNode);
      return;
    }
    await delegateToCommunityNode(selectedCommunityNode);
  };
  const acceptConsentFromDialog = async () => {
    if (!consentDialogNode || !consentDialogView || !onAcceptCommunityNodeConsents) return;
    const documents = consentDialogView.policies.map((policy) => ({
      policy_slug: policy.policySlug,
      policy_version: policy.policyVersion,
      policy_snapshot_revision: policy.policySnapshotRevision ?? null,
    }));
    if (documents.length === 0) return;
    setConsentBusy(true);
    try {
      await onAcceptCommunityNodeConsents(consentDialogNode.baseUrl, documents);
      setConsentDialogNodeBaseUrl(null);
      setConsentDialogLoadedBaseUrl(null);
      await delegateToCommunityNode(consentDialogNode);
    } catch (acceptError) {
      setError(acceptError instanceof Error ? acceptError.message : t('hosting.error'));
    } finally {
      setConsentBusy(false);
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
            <span>{t('hosting.communityNode')}</span>
            <Select
              aria-label={t('hosting.communityNode')}
              value={selectedNodeBaseUrl}
              disabled={pending || selectableCommunityNodes.length === 0}
              onChange={(event) => setSelectedNodeBaseUrl(event.target.value)}
            >
              <option value=''>{t('hosting.chooseCommunityNode')}</option>
              {savedCommunityNodes.map((node) => (
                <option key={node.id} value={node.baseUrl} disabled={!node.nodeId?.trim()}>
                  {node.nodeName?.trim() || node.baseUrl}
                </option>
              ))}
            </Select>
          </Label>
          {selectedCommunityNode ? (
            <small className='break-all font-mono'>
              {selectedCommunityNode.nodeId} · {selectedCommunityNode.baseUrl}
            </small>
          ) : (
            <Notice>
              {savedCommunityNodes.length > 0
                ? t('hosting.manifestRequired')
                : t('hosting.communityNodeRequired')}
              {onOpenCommunityNodeSettings ? (
                <Button type='button' variant='ghost' onClick={onOpenCommunityNodeSettings}>
                  {t('hosting.openCommunityNodeSettings')}
                </Button>
              ) : null}
            </Notice>
          )}
          <Button
            type='button'
            variant='secondary'
            disabled={pending || !selectedCommunityNode}
            onClick={() => void requestCommunityNodeDelegation()}
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
      {consentDialogNode && consentDialogView ? (
        <CommunityNodeConsentDialog
          open={consentDialogNodeBaseUrl != null}
          onOpenChange={(open) => {
            if (!open) {
              setConsentDialogNodeBaseUrl(null);
              setConsentDialogLoadedBaseUrl(null);
              setConsentDialogFetchError(null);
            }
          }}
          baseUrl={consentDialogNode.baseUrl}
          consent={consentDialogView}
          busy={consentBusy}
          onAccept={() => void acceptConsentFromDialog()}
          onRetry={() => void openConsentDialog(consentDialogNode)}
        />
      ) : null}
    </Card>
  );
}
