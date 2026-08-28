import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { DesktopApi } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

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
import { Textarea } from '@/components/ui/textarea';

/// 各項目の上限(Unicode コードポイント数)。サーバ側の契約と同じ値(#802 / ADR 0039)。
export const TESTER_FEEDBACK_MAX_CHARS = 2000;

type TesterFeedbackDialogProps = {
  api: DesktopApi;
  open: boolean;
  eligibleNodeBaseUrls: readonly string[];
  onOpenChange: (open: boolean) => void;
  onOpenCommunityNodeSettings: () => void;
};

type FeedbackFieldKey = 'whatAttempted' | 'whatHappened' | 'whatSeemedWrong';

function feedbackErrorKey(error: unknown): string {
  if (!(error instanceof InvokeError)) return 'requestFailed';
  if (error.code === 'TESTER_FEEDBACK_NOT_CONFIGURED') return 'notConfigured';
  if (error.code === 'INVALID_TESTER_FEEDBACK') return 'invalidFeedback';
  if (error.code === 'AUTH_REQUIRED' || error.status === 401) return 'authRequired';
  if (error.code === 'CONSENT_REQUIRED' || error.status === 403) return 'consentRequired';
  return 'requestFailed';
}

/// 文字数は byte / UTF-16 長ではなく Unicode コードポイント数で数える(ADR 0039)。
function charCount(text: string): number {
  return [...text].length;
}

export function TesterFeedbackDialog({
  api,
  open,
  eligibleNodeBaseUrls,
  onOpenChange,
  onOpenCommunityNodeSettings,
}: TesterFeedbackDialogProps) {
  const { t } = useTranslation(['shell', 'common']);
  const [selectedNode, setSelectedNode] = useState('');
  const [fields, setFields] = useState<Record<FeedbackFieldKey, string>>({
    whatAttempted: '',
    whatHappened: '',
    whatSeemedWrong: '',
  });
  const [pending, setPending] = useState(false);
  const [referenceId, setReferenceId] = useState<string | null>(null);
  const [errorKey, setErrorKey] = useState<string | null>(null);
  const requestVersionRef = useRef(0);

  // 適格一覧は定期更新のたびに新しい配列になり得るため、参照ではなく内容の変化で初期化する。
  const eligibleKey = JSON.stringify(eligibleNodeBaseUrls);
  useEffect(() => {
    requestVersionRef.current += 1;
    setSelectedNode((JSON.parse(eligibleKey) as string[])[0] ?? '');
    setPending(false);
    setReferenceId(null);
    setErrorKey(null);
  }, [eligibleKey, open]);
  useEffect(() => {
    if (open) {
      setFields({ whatAttempted: '', whatHappened: '', whatSeemedWrong: '' });
    }
  }, [open]);
  const selectedNodeEligible = selectedNode !== '' && eligibleNodeBaseUrls.includes(selectedNode);

  const overLimit = (Object.values(fields) as string[]).some(
    (value) => charCount(value) > TESTER_FEEDBACK_MAX_CHARS
  );
  const anyEmpty = (Object.values(fields) as string[]).some((value) => value.trim() === '');

  function updateField(key: FeedbackFieldKey, value: string) {
    setFields((current) => ({ ...current, [key]: value }));
    setReferenceId(null);
    setErrorKey(null);
  }

  function selectNode(baseUrl: string) {
    requestVersionRef.current += 1;
    setSelectedNode(baseUrl);
    setReferenceId(null);
    setErrorKey(null);
  }

  async function submit() {
    if (!selectedNodeEligible || anyEmpty || overLimit) return;
    const requestVersion = requestVersionRef.current + 1;
    requestVersionRef.current = requestVersion;
    setPending(true);
    setReferenceId(null);
    setErrorKey(null);
    try {
      const response = await api.submitCommunityNodeTesterFeedback({
        base_url: selectedNode,
        what_attempted: fields.whatAttempted,
        what_happened: fields.whatHappened,
        what_seemed_wrong: fields.whatSeemedWrong,
      });
      if (requestVersionRef.current !== requestVersion) return;
      setReferenceId(response.reference_id ?? '');
      setFields({ whatAttempted: '', whatHappened: '', whatSeemedWrong: '' });
    } catch (error) {
      if (requestVersionRef.current !== requestVersion) return;
      setErrorKey(feedbackErrorKey(error));
    } finally {
      if (requestVersionRef.current === requestVersion) setPending(false);
    }
  }

  const fieldDefs: Array<{ key: FeedbackFieldKey; label: string }> = [
    { key: 'whatAttempted', label: t('shell:testerFeedback.whatAttemptedLabel') },
    { key: 'whatHappened', label: t('shell:testerFeedback.whatHappenedLabel') },
    { key: 'whatSeemedWrong', label: t('shell:testerFeedback.whatSeemedWrongLabel') },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{t('shell:testerFeedback.title')}</DialogTitle>
          <DialogDescription>{t('shell:testerFeedback.description')}</DialogDescription>
        </DialogHeader>
        <DialogBody>
          <div className='extended-module-stack'>
            {eligibleNodeBaseUrls.length === 0 ? (
              <Notice>
                <p>{t('shell:testerFeedback.noEligibleNode')}</p>
                <Button type='button' variant='secondary' onClick={onOpenCommunityNodeSettings}>
                  {t('shell:testerFeedback.openSettings')}
                </Button>
              </Notice>
            ) : (
              <Label>
                <span>{t('shell:testerFeedback.nodeLabel')}</span>
                <Select
                  value={selectedNode}
                  disabled={pending}
                  onChange={(event) => selectNode(event.target.value)}
                >
                  {eligibleNodeBaseUrls.map((baseUrl) => (
                    <option key={baseUrl} value={baseUrl}>
                      {baseUrl}
                    </option>
                  ))}
                </Select>
              </Label>
            )}

            {fieldDefs.map(({ key, label }) => (
              <div key={key}>
                <Label>
                  <span>{label}</span>
                  <Textarea
                    rows={3}
                    value={fields[key]}
                    disabled={pending}
                    aria-invalid={charCount(fields[key]) > TESTER_FEEDBACK_MAX_CHARS}
                    onChange={(event) => updateField(key, event.target.value)}
                  />
                </Label>
                <p className='muted'>
                  {t('shell:testerFeedback.charCount', {
                    current: charCount(fields[key]),
                    max: TESTER_FEEDBACK_MAX_CHARS,
                  })}
                </p>
              </div>
            ))}

            <p className='muted'>{t('shell:testerFeedback.metadataNotice')}</p>

            {overLimit ? (
              <Notice tone='warning'>
                {t('shell:testerFeedback.overLimit', { max: TESTER_FEEDBACK_MAX_CHARS })}
              </Notice>
            ) : null}
            {referenceId !== null ? (
              <Notice tone='accent'>{t('shell:testerFeedback.success', { referenceId })}</Notice>
            ) : null}
            {errorKey ? (
              <Notice tone='destructive'>{t(`shell:testerFeedback.errors.${errorKey}`)}</Notice>
            ) : null}

            <div className='ui-dialog-footer'>
              <Button type='button' variant='secondary' onClick={() => onOpenChange(false)}>
                {t('common:actions.close')}
              </Button>
              <Button
                type='button'
                disabled={pending || !selectedNodeEligible || anyEmpty || overLimit}
                onClick={() => void submit()}
              >
                {pending ? t('shell:testerFeedback.submitting') : t('shell:testerFeedback.submit')}
              </Button>
            </div>
          </div>
        </DialogBody>
      </DialogContent>
    </Dialog>
  );
}
