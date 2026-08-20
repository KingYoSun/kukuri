import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { DesktopApi, IndexingRequestStatus } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';
import { topicDisplayName } from '@/lib/topicId';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { Notice } from '@/components/ui/notice';
import { Select } from '@/components/ui/select';

export type CommunityIndexingTarget =
  | { kind: 'public_topic'; topicId: string }
  | { kind: 'private_channel'; topicId: string; channelId: string; channelLabel: string };

type CommunityIndexingRequestDialogProps = {
  api: DesktopApi;
  target: CommunityIndexingTarget | null;
  eligibleNodeBaseUrls: readonly string[];
  onOpenChange: (open: boolean) => void;
  onOpenCommunityNodeSettings: () => void;
};

function requestErrorKey(error: unknown): string {
  if (!(error instanceof InvokeError)) return 'requestFailed';
  // #713: 索引を提供しない・停止中のノードは申請を受け付けない(サーバ側の門)。
  if (error.code === 'INDEXING_REQUEST_NOT_CONFIGURED') return 'requestNotConfigured';
  if (error.code === 'INDEXING_REQUEST_NOT_ACTIVATED') return 'requestNotActivated';
  if (error.code === 'CHANNEL_INDEXING_NOT_CONFIGURED') return 'notConfigured';
  if (error.code === 'CHANNEL_SECRET_CONFLICT') return 'secretConflict';
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) return 'authRequired';
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) return 'consentRequired';
  return 'requestFailed';
}

export function CommunityIndexingRequestDialog({
  api,
  target,
  eligibleNodeBaseUrls,
  onOpenChange,
  onOpenCommunityNodeSettings,
}: CommunityIndexingRequestDialogProps) {
  const { t } = useTranslation(['shell', 'common']);
  const [selectedNode, setSelectedNode] = useState('');
  const [confirmed, setConfirmed] = useState(false);
  const [pending, setPending] = useState(false);
  const [status, setStatus] = useState<IndexingRequestStatus | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const requestVersionRef = useRef(0);

  // 適格一覧は定期更新のたびに新しい配列になり得るため、参照ではなく内容の変化で初期化する(#698)。
  const eligibleKey = JSON.stringify(eligibleNodeBaseUrls);
  useEffect(() => {
    requestVersionRef.current += 1;
    setSelectedNode((JSON.parse(eligibleKey) as string[])[0] ?? '');
    setConfirmed(false);
    setPending(false);
    setStatus(null);
    setErrorKey(null);
  }, [eligibleKey, target]);
  const selectedNodeEligible = selectedNode !== '' && eligibleNodeBaseUrls.includes(selectedNode);

  const privateTarget = target?.kind === 'private_channel';
  const targetLabel = target
    ? privateTarget
      ? target.channelLabel
      : topicDisplayName(target.topicId)
    : '';

  function selectNode(baseUrl: string) {
    requestVersionRef.current += 1;
    setSelectedNode(baseUrl);
    setConfirmed(false);
    setStatus(null);
    setErrorKey(null);
  }

  async function submit() {
    // 選択ノードが適格一覧から外れた瞬間から申請(非公開チャンネルでは秘密値)を送らない(#698)。
    if (!target || !selectedNodeEligible || (privateTarget && !confirmed)) return;
    const requestVersion = requestVersionRef.current + 1;
    requestVersionRef.current = requestVersion;
    if (privateTarget) setConfirmed(false);
    setPending(true);
    setStatus(null);
    setErrorKey(null);
    try {
      const response = await api.submitCommunityNodeIndexingRequest({
        base_url: selectedNode,
        scope_kind: target.kind,
        topic_id: target.topicId,
        channel_id: privateTarget ? target.channelId : null,
        confirm_private_channel_secret_disclosure: privateTarget && confirmed,
      });
      if (requestVersionRef.current !== requestVersion) return;
      setStatus(response.status);
    } catch (error) {
      if (requestVersionRef.current !== requestVersion) return;
      setErrorKey(requestErrorKey(error));
    } finally {
      if (requestVersionRef.current === requestVersion) setPending(false);
    }
  }

  return (
    <Dialog open={target !== null} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('shell:indexingRequest.title')}</DialogTitle>
          <DialogDescription>
            {t('shell:indexingRequest.target', { target: targetLabel })}
          </DialogDescription>
        </DialogHeader>
        <DialogBody>
          <div className='extended-module-stack'>
            {eligibleNodeBaseUrls.length === 0 ? (
              <Notice>
                <p>{t('shell:indexingRequest.noEligibleNode')}</p>
                <Button type='button' variant='secondary' onClick={onOpenCommunityNodeSettings}>
                  {t('shell:indexingRequest.openSettings')}
                </Button>
              </Notice>
            ) : (
              <Label>
                <span>{t('shell:indexingRequest.nodeLabel')}</span>
                <Select
                  value={selectedNode}
                  disabled={pending}
                  onChange={(event) => selectNode(event.target.value)}
                >
                  {eligibleNodeBaseUrls.map((baseUrl) => (
                    <option key={baseUrl} value={baseUrl}>{baseUrl}</option>
                  ))}
                </Select>
              </Label>
            )}

            {privateTarget ? (
              <Notice tone='warning'>
                <label className='flex items-start gap-3'>
                  <input
                    type='checkbox'
                    className='mt-1 size-4 shrink-0'
                    checked={confirmed}
                    disabled={pending}
                    onChange={(event) => setConfirmed(event.target.checked)}
                  />
                  <span className='min-w-0'>{t('shell:indexingRequest.privateConfirmation')}</span>
                </label>
                <p className='mb-0 mt-2'>{t('shell:indexingRequest.privateWarning')}</p>
              </Notice>
            ) : (
              <Notice>{t('shell:indexingRequest.publicNotice')}</Notice>
            )}
            <p className='muted'>{t('shell:indexingRequest.gateNotice')}</p>

            {status ? (
              <Notice tone={status === 'rejected' ? 'destructive' : 'accent'}>
                {t(`shell:indexingRequest.status.${status}`)}
              </Notice>
            ) : null}
            {errorKey ? <Notice tone='destructive'>{t(`shell:indexingRequest.errors.${errorKey}`)}</Notice> : null}

            <div className='ui-dialog-footer'>
              <Button type='button' variant='secondary' onClick={() => onOpenChange(false)}>
                {t('common:actions.close')}
              </Button>
              <Button
                type='button'
                disabled={pending || !selectedNodeEligible || (privateTarget && !confirmed)}
                onClick={() => void submit()}
              >
                {pending ? t('shell:indexingRequest.submitting') : t('shell:indexingRequest.submit')}
              </Button>
            </div>
          </div>
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}
