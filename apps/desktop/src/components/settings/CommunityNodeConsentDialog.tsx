import { useTranslation } from 'react-i18next';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Notice } from '@/components/ui/notice';

import { type CommunityNodeConsentView } from './types';

type CommunityNodeConsentDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  baseUrl: string;
  consent: CommunityNodeConsentView;
  busy: boolean;
  onAccept: () => void;
  // #857: 取得失敗（オフライン等）時の再試行。
  onRetry: () => void;
  // #857: 同意の撤回。同意済みのときだけ渡す。
  onWithdraw?: () => void;
};

// #857: Node 同意モーダル。提示内容は認証不要の公開 policy カタログから組み立て、
// 認証(JWT 発行)は同意成立後にのみ始まる。不同意(閉じる)でも非 Node 機能は使える。
export function CommunityNodeConsentDialog({
  open,
  onOpenChange,
  baseUrl,
  consent,
  busy,
  onAccept,
  onRetry,
  onWithdraw,
}: CommunityNodeConsentDialogProps) {
  const { t } = useTranslation(['common', 'settings']);

  const acceptDisabled = busy || !consent.loaded || consent.allRequiredAccepted;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-h-[88vh] w-[min(40rem,92vw)] overflow-hidden'>
        <DialogHeader>
          <DialogTitle>{t('settings:communityNode.consent.title')}</DialogTitle>
          <p className='break-all font-mono text-xs text-[var(--muted-foreground)]'>{baseUrl}</p>
        </DialogHeader>

        <DialogBody className='max-h-[60vh] space-y-4 overflow-y-auto'>
          <p className='text-sm leading-6 text-[var(--muted-foreground)]'>
            {t('settings:communityNode.consent.intro')}
          </p>

          {consent.loading ? (
            <Notice aria-live='polite'>{t('settings:communityNode.consent.loading')}</Notice>
          ) : null}

          {consent.loadError ? (
            <Notice tone='destructive'>
              <div className='flex flex-wrap items-center justify-between gap-3'>
                <span>{t('settings:communityNode.consent.loadFailed')}</span>
                <Button variant='secondary' type='button' disabled={busy} onClick={onRetry}>
                  {t('common:actions.retry')}
                </Button>
              </div>
              <small className='font-mono'>{consent.loadError}</small>
            </Notice>
          ) : null}

          {consent.withdrawn ? (
            <Notice tone='warning'>{t('settings:communityNode.consent.withdrawnNotice')}</Notice>
          ) : null}

          {consent.loaded && consent.hasPendingUpdate ? (
            <Notice tone='warning'>{t('settings:communityNode.consent.updatedNotice')}</Notice>
          ) : null}

          {consent.loaded && consent.policies.length === 0 ? (
            <Notice>{t('settings:communityNode.consent.noPolicies')}</Notice>
          ) : null}

          {consent.loaded
            ? consent.policies.map((policy) => (
                <section
                  key={policy.policySlug}
                  className='space-y-3 rounded-[16px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-4'
                >
              <div className='flex flex-wrap items-start justify-between gap-2'>
                <div className='min-w-0 space-y-1'>
                  <h5 className='break-words text-sm font-semibold text-foreground'>
                    {policy.title}
                  </h5>
                  <p className='text-xs font-semibold uppercase tracking-[0.08em] text-[var(--muted-foreground)]'>
                    v{policy.policyVersion}
                  </p>
                </div>
                <div className='flex flex-wrap items-center gap-2'>
                  {policy.required ? (
                    <Badge tone='accent'>{t('settings:communityNode.consent.required')}</Badge>
                  ) : (
                    <Badge tone='neutral'>{t('settings:communityNode.consent.optional')}</Badge>
                  )}
                  {policy.updated ? (
                    <Badge tone='warning'>{t('settings:communityNode.consent.updatedBadge')}</Badge>
                  ) : null}
                </div>
              </div>

              {policy.updated && policy.previouslyAcceptedVersion != null ? (
                <p className='text-xs text-[var(--muted-foreground)]'>
                  {t('settings:communityNode.consent.updatedDetail', {
                    previous: policy.previouslyAcceptedVersion,
                    current: policy.policyVersion,
                  })}
                </p>
              ) : null}

              <p className='text-sm text-[var(--muted-foreground)]'>
                {policy.acceptedAtLabel
                  ? t('settings:communityNode.consent.acceptedAt', {
                      timestamp: policy.acceptedAtLabel,
                    })
                  : t('settings:communityNode.consent.notAccepted')}
              </p>

              {policy.body.trim() ? (
                <p className='whitespace-pre-wrap break-words text-sm leading-6 text-foreground'>
                  {policy.body}
                </p>
              ) : (
                <p className='text-sm italic text-[var(--muted-foreground)]'>
                  {t('settings:communityNode.consent.noBody')}
                </p>
              )}
                </section>
              ))
            : null}
        </DialogBody>

        <DialogFooter className='flex flex-wrap justify-end gap-2'>
          {onWithdraw && consent.hasLocalConsent ? (
            <Button variant='secondary' disabled={busy} onClick={onWithdraw}>
              {t('settings:communityNode.consent.withdraw')}
            </Button>
          ) : null}
          <Button variant='secondary' onClick={() => onOpenChange(false)}>
            {consent.allRequiredAccepted
              ? t('common:actions.close')
              : t('settings:communityNode.consent.decline')}
          </Button>
          <Button disabled={acceptDisabled} onClick={onAccept}>
            {consent.allRequiredAccepted
              ? t('settings:communityNode.consent.allAccepted')
              : t('common:actions.accept')}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
