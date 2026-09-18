import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import type { AuthorTrustGateView } from '@/shell/authorTrustGates';

/// #1061: 採用 CN の信頼値で折りたたんだ投稿の代替表示（ADR 0026 §8.4）。
///
/// 断定ラベルは出さず、理由の種類と採用した CN を示し、この投稿だけを表示する導線と、
/// 作者詳細（常に表示する設定と、ブロック・ミュートの操作）への導線を残す。
export type AuthorTrustGateNoticeProps = {
  gate: AuthorTrustGateView;
  onReveal: () => void;
  onOpenAuthor?: (authorPubkey: string) => void;
};

export function AuthorTrustGateNotice({ gate, onReveal, onOpenAuthor }: AuthorTrustGateNoticeProps) {
  const { t } = useTranslation(['common']);
  const reasons = gate.reasons
    .map((reason) => t(`common:feed.authorTrustGate.reasons.${reason}`))
    .join(' / ');

  return (
    <div
      className='space-y-2 rounded-[16px] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] p-3'
      data-testid='author-trust-gate-notice'
      role='status'
    >
      <p className='text-sm text-[var(--muted-foreground)]'>
        {gate.fromRepostSource
          ? t('common:feed.authorTrustGate.repostSourceSummary')
          : t('common:feed.authorTrustGate.summary')}
      </p>
      {reasons ? (
        <p className='text-xs text-[var(--muted-foreground)]'>
          {t('common:feed.authorTrustGate.reasonsLabel', { reasons })}
        </p>
      ) : null}
      {gate.nodeBaseUrl ? (
        <p className='break-all font-mono text-xs text-[var(--muted-foreground)]'>
          {t('common:feed.authorTrustGate.node', { node: gate.nodeBaseUrl })}
        </p>
      ) : null}
      <div className='flex flex-wrap gap-2'>
        <Button variant='secondary' onClick={onReveal} data-testid='author-trust-gate-reveal'>
          {t('common:feed.authorTrustGate.reveal')}
        </Button>
        {onOpenAuthor ? (
          <Button
            variant='secondary'
            onClick={() => onOpenAuthor(gate.authorPubkey)}
            data-testid='author-trust-gate-open-author'
          >
            {t('common:feed.authorTrustGate.openAuthor')}
          </Button>
        ) : null}
      </div>
    </div>
  );
}
