import { type ReactNode, useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { AlertTriangle, ExternalLink, ShieldAlert } from 'lucide-react';

import {
  REPORT_REASONS,
  type ReportReason,
  type ReportRoutingCandidate,
  type ReportRoutingPlan,
  isCriticalSafetyReason,
} from '@/lib/api/reportRouting';
import { type SubmitCommunityNodeReportResult } from '@/lib/api';

import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Notice } from '@/components/ui/notice';

/// 通報対象の種別。provenance を持ち得る surface（post / profile / media 等）に対応する。
export type ReportSubjectKind = 'post' | 'profile' | 'media' | 'search_result' | 'recommendation';

export type ReportRoutingSubject = {
  kind: ReportSubjectKind;
  id: string;
  /// 表示用の短いラベル（author / 抜粋など）。送信内容には含めない。
  label?: string;
};

export type ReportSubmitInput = {
  candidate: ReportRoutingCandidate;
  reason: ReportReason;
  details: string;
  reporterContact: string;
};

export type ReportRoutingDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  subject: ReportRoutingSubject;
  plan: ReportRoutingPlan;
  /// report endpoint を持つ候補への送信。contact のみの候補は onCopyContact で案内する。
  onSubmit: (input: ReportSubmitInput) => Promise<SubmitCommunityNodeReportResult>;
  /// abuse contact（mailto / copyable）の案内。
  onCopyContact?: (value: string) => void;
  /// provenance 不明 / 通報先未解決時に出す local action（block / mute / local hide）導線。
  localActions?: ReactNode;
};

function nodeHost(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url.replace(/^https?:\/\//, '');
  }
}

function candidateKey(candidate: ReportRoutingCandidate): string {
  return `${candidate.target.nodeBaseUrl} ${candidate.target.capability}`;
}

export function ReportRoutingDialog({
  open,
  onOpenChange,
  subject,
  plan,
  onSubmit,
  onCopyContact,
  localActions,
}: ReportRoutingDialogProps) {
  const { t } = useTranslation(['shell', 'common']);
  const { candidates } = plan;

  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [reason, setReason] = useState<ReportReason>('spam');
  const [details, setDetails] = useState('');
  const [reporterContact, setReporterContact] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<SubmitCommunityNodeReportResult | null>(null);

  // ダイアログを開くたびに入力状態を初期化する。
  useEffect(() => {
    if (open) {
      setSelectedKey(candidates.length > 0 ? candidateKey(candidates[0]) : null);
      setReason('spam');
      setDetails('');
      setReporterContact('');
      setSubmitting(false);
      setError(null);
      setResult(null);
    }
  }, [open, candidates]);

  const selectedCandidate = useMemo(
    () => candidates.find((candidate) => candidateKey(candidate) === selectedKey) ?? null,
    [candidates, selectedKey],
  );

  const isCriticalSafety = isCriticalSafetyReason(reason);

  const handleSubmit = async () => {
    if (!selectedCandidate) {
      return;
    }
    const contact = selectedCandidate.contact;
    // endpoint が無い候補は POST せず、abuse contact を案内する（#310 初期実装方針）。
    if (contact.kind === 'contact') {
      onCopyContact?.(contact.value);
      return;
    }
    if (contact.kind !== 'endpoint') {
      return;
    }
    setSubmitting(true);
    setError(null);
    try {
      const submitted = await onSubmit({ candidate: selectedCandidate, reason, details, reporterContact });
      setResult(submitted);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setSubmitting(false);
    }
  };

  const canRoute = candidates.length > 0;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='report-routing-dialog'>
        <DialogHeader>
          <DialogTitle>{t('report.title')}</DialogTitle>
          <DialogDescription>{t('report.boundaryNotice')}</DialogDescription>
        </DialogHeader>
        <DialogBody className='report-routing-body'>
          <p className='report-subject'>
            {t(`report.subject.${subject.kind}`)}
            {subject.label ? ` · ${subject.label}` : ''}
          </p>

          {/* 通報先が kukuri 全体ではないことを常に明示する。 */}
          <Notice tone='accent' className='report-boundary-notice'>
            <p>{t('report.boundaryDetail')}</p>
            <p className='report-identity-boundary'>{t('report.identityBoundaryNote')}</p>
          </Notice>

          {result ? (
            <Notice tone='accent' className='report-result'>
              <p>{t('report.success')}</p>
              {result.reference_id ? (
                <p className='report-result-reference'>
                  {t('report.referenceId', { id: result.reference_id })}
                </p>
              ) : null}
            </Notice>
          ) : canRoute ? (
            <>
              <fieldset className='report-target-list'>
                <legend>{t('report.targetsHeading')}</legend>
                {candidates.map((candidate) => {
                  const key = candidateKey(candidate);
                  const { target, contact } = candidate;
                  return (
                    <label key={key} className='report-target-option'>
                      <input
                        type='radio'
                        name='report-target'
                        value={key}
                        checked={selectedKey === key}
                        onChange={() => setSelectedKey(key)}
                      />
                      <span className='report-target-main'>
                        <span className='report-target-host'>{nodeHost(target.nodeBaseUrl)}</span>
                        <span className='report-target-capability'>
                          {t(`report.capability.${target.capability}`)}
                        </span>
                        <span className='report-target-contact'>
                          {contact.kind === 'endpoint'
                            ? t('report.contact.endpoint')
                            : contact.kind === 'contact'
                              ? t('report.contact.mailto', { contact: contact.value })
                              : null}
                        </span>
                        {target.policyUrl ? (
                          <a
                            className='report-target-policy'
                            href={target.policyUrl}
                            target='_blank'
                            rel='noreferrer'
                          >
                            <ExternalLink className='size-3.5' aria-hidden='true' />
                            {t('report.openPolicy')}
                          </a>
                        ) : null}
                      </span>
                    </label>
                  );
                })}
              </fieldset>

              <label className='report-field'>
                <span>{t('report.reasonLabel')}</span>
                <select
                  className='report-reason-select'
                  value={reason}
                  onChange={(event) => setReason(event.target.value as ReportReason)}
                >
                  {REPORT_REASONS.map((value) => (
                    <option key={value} value={value}>
                      {t(`report.reasons.${value}`)}
                    </option>
                  ))}
                </select>
              </label>

              {isCriticalSafety ? (
                <Notice tone='warning' className='report-critical-safety'>
                  <ShieldAlert className='size-4' aria-hidden='true' />
                  <span>{t('report.criticalSafetyNote')}</span>
                </Notice>
              ) : null}

              <label className='report-field'>
                <span>{t('report.detailsLabel')}</span>
                <textarea
                  className='report-details-input'
                  rows={3}
                  value={details}
                  placeholder={t('report.detailsPlaceholder')}
                  onChange={(event) => setDetails(event.target.value)}
                />
              </label>

              <label className='report-field'>
                <span>{t('report.reporterContactLabel')}</span>
                <input
                  className='report-reporter-contact-input'
                  type='text'
                  value={reporterContact}
                  placeholder={t('report.reporterContactPlaceholder')}
                  onChange={(event) => setReporterContact(event.target.value)}
                />
                <small className='report-field-hint'>{t('report.reporterContactHint')}</small>
              </label>

              {error ? (
                <Notice tone='destructive' className='report-error'>
                  {error}
                </Notice>
              ) : null}
            </>
          ) : (
            // provenance 不明 / 通報先未解決：default node へ向けず local action のみ案内する。
            <Notice tone='warning' className='report-unresolved'>
              <AlertTriangle className='size-4' aria-hidden='true' />
              <div className='report-unresolved-body'>
                <p className='report-unresolved-title'>
                  {plan.provenanceUnknown
                    ? t('report.unknownTitle')
                    : t('report.observedUnresolvedTitle')}
                </p>
                <p>
                  {plan.provenanceUnknown
                    ? t('report.unknownBody')
                    : t('report.observedUnresolvedBody')}
                </p>
                <p className='report-local-actions-hint'>{t('report.localActionsHint')}</p>
                {localActions ? <div className='report-local-actions'>{localActions}</div> : null}
              </div>
            </Notice>
          )}
        </DialogBody>
        <DialogFooter className='report-routing-footer'>
          <Button variant='secondary' type='button' onClick={() => onOpenChange(false)}>
            {t(result ? 'common:actions.close' : 'common:actions.cancel')}
          </Button>
          {!result && canRoute && selectedCandidate ? (
            <Button
              type='button'
              disabled={submitting}
              onClick={handleSubmit}
              aria-label={t('report.submit')}
            >
              {selectedCandidate.contact.kind === 'endpoint'
                ? submitting
                  ? t('report.submitting')
                  : t('report.submit')
                : t('report.copyContact')}
            </Button>
          ) : null}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
