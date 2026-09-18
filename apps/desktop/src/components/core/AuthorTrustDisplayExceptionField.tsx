import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Notice } from '@/components/ui/notice';

/// #1061: 採用 CN の信頼値で折りたたむ対象から、この著者を外す設定（ADR 0026 §8.4）。
///
/// 端末内の設定で、ブロック・ミュートや CN の評価そのものは変えない。
export type AuthorTrustDisplayExceptionFieldProps = {
  authorPubkey: string;
  /// 現在の設定と、折りたたみ対象かどうかを読む。
  loadAlwaysVisible: (authorPubkey: string) => Promise<boolean>;
  setAlwaysVisible: (authorPubkey: string, alwaysVisible: boolean) => Promise<boolean>;
};

export function AuthorTrustDisplayExceptionField({
  authorPubkey,
  loadAlwaysVisible,
  setAlwaysVisible,
}: AuthorTrustDisplayExceptionFieldProps) {
  const { t } = useTranslation(['profile']);
  const [alwaysVisible, setValue] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState(false);

  const load = useCallback(async () => {
    try {
      setValue(await loadAlwaysVisible(authorPubkey));
    } catch {
      setValue(null);
    }
  }, [authorPubkey, loadAlwaysVisible]);

  useEffect(() => {
    void load();
  }, [load]);

  if (alwaysVisible === null) return null;

  return (
    <section
      className='mt-4 space-y-2 rounded-[16px] border border-[var(--border-subtle)] p-4'
      data-testid='author-trust-display-exception'
    >
      <label className='flex min-w-0 items-start gap-3 text-sm font-medium text-foreground'>
        <input
          type='checkbox'
          className='mt-0.5 size-4 shrink-0'
          checked={alwaysVisible}
          disabled={busy}
          data-testid='author-trust-display-exception-toggle'
          onChange={(event) => {
            const next = event.currentTarget.checked;
            setBusy(true);
            setError(false);
            void setAlwaysVisible(authorPubkey, next)
              .then((value) => setValue(value))
              .catch(() => setError(true))
              .finally(() => setBusy(false));
          }}
        />
        <span className='min-w-0'>{t('profile:authorTrustDisplay.label')}</span>
      </label>
      <p className='text-sm text-[var(--muted-foreground)]'>
        {t('profile:authorTrustDisplay.description')}
      </p>
      {error ? (
        <Notice tone='destructive' role='alert'>
          {t('profile:authorTrustDisplay.failed')}
        </Notice>
      ) : null}
    </section>
  );
}
