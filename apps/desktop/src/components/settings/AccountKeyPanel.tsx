import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import type {
  AccountKeyExport,
  AccountKeyImportPreview,
  AccountRecord,
  AccountsSnapshot,
} from '@/lib/api/types.generated';

import { Button } from '@/components/ui/button';
import { Card, CardHeader } from '@/components/ui/card';
import { Field } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';
import {
  exportAccountKey,
  importAccountKey,
  listAccounts,
  previewAccountKeyImport,
  switchAccount,
} from '@/lib/api/identity';
import { copyTextToClipboard } from '@/lib/utils';
import { COLUMN_DRAFT_STORAGE_KEY } from '@/shell/columnDraftPersistence';

const MIN_PASSPHRASE_CHARS = 8;

function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}

// #859: アカウント鍵の export / import と複数アカウント管理(ADR 0047)。
// 平文秘密鍵はこのパネルにも IPC にも一切現れない。export は暗号化 envelope のみ。
export function AccountKeyPanel() {
  const { t } = useTranslation(['settings']);

  const [accounts, setAccounts] = useState<AccountsSnapshot | null>(null);
  const [accountsError, setAccountsError] = useState<string | null>(null);

  const [exportAcknowledged, setExportAcknowledged] = useState(false);
  const [exportPassphrase, setExportPassphrase] = useState('');
  const [exportPassphraseConfirm, setExportPassphraseConfirm] = useState('');
  const [exportPending, setExportPending] = useState(false);
  const [exportResult, setExportResult] = useState<AccountKeyExport | null>(null);
  const [exportError, setExportError] = useState<string | null>(null);
  const [exportCopied, setExportCopied] = useState(false);

  const [importText, setImportText] = useState('');
  const [importPreview, setImportPreview] = useState<AccountKeyImportPreview | null>(null);
  const [importPassphrase, setImportPassphrase] = useState('');
  const [importLabel, setImportLabel] = useState('');
  const [importPending, setImportPending] = useState(false);
  const [importResult, setImportResult] = useState<AccountRecord | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

  const [switchPendingId, setSwitchPendingId] = useState<string | null>(null);
  const [switchError, setSwitchError] = useState<string | null>(null);

  const refreshAccounts = useCallback(async () => {
    try {
      setAccounts(await listAccounts());
      setAccountsError(null);
    } catch (error) {
      setAccountsError(errorMessage(error));
    }
  }, []);

  useEffect(() => {
    void refreshAccounts();
  }, [refreshAccounts]);

  const passphraseTooShort =
    exportPassphrase.length > 0 && exportPassphrase.length < MIN_PASSPHRASE_CHARS;
  const passphraseMismatch =
    exportPassphraseConfirm.length > 0 && exportPassphrase !== exportPassphraseConfirm;
  const exportReady =
    exportAcknowledged &&
    exportPassphrase.length >= MIN_PASSPHRASE_CHARS &&
    exportPassphrase === exportPassphraseConfirm &&
    !exportPending;

  const handleExport = async () => {
    setExportPending(true);
    setExportError(null);
    setExportCopied(false);
    try {
      setExportResult(await exportAccountKey(exportPassphrase));
    } catch (error) {
      setExportError(errorMessage(error));
    } finally {
      setExportPending(false);
    }
  };

  const handleCopyExport = async () => {
    if (!exportResult) return;
    // 暗号化 envelope のコピー。平文秘密鍵はクリップボードに載らない。
    const copied = await copyTextToClipboard(exportResult.export);
    setExportCopied(copied);
  };

  const handlePreviewImport = async () => {
    setImportError(null);
    setImportResult(null);
    try {
      setImportPreview(await previewAccountKeyImport(importText.trim()));
    } catch (error) {
      setImportPreview(null);
      setImportError(errorMessage(error));
    }
  };

  const handleImport = async () => {
    setImportPending(true);
    setImportError(null);
    try {
      const record = await importAccountKey(
        importText.trim(),
        importPassphrase,
        importLabel.trim() || undefined
      );
      setImportResult(record);
      setImportPassphrase('');
      await refreshAccounts();
    } catch (error) {
      setImportError(errorMessage(error));
    } finally {
      setImportPending(false);
    }
  };

  const handleSwitch = async (accountId: string) => {
    setSwitchPendingId(accountId);
    setSwitchError(null);
    try {
      await switchAccount(accountId);
      // 下書きは本人性に紐づく内容のため、他アカウントへ持ち越さない。
      try {
        window.localStorage.removeItem(COLUMN_DRAFT_STORAGE_KEY);
      } catch {
        // localStorage が使えない環境では持ち越し防止をあきらめる。
      }
      // runtime は切替済み。UI 状態を新アカウントで作り直すために再読み込みする
      // (アプリ・プロセスの再起動は不要)。
      window.location.reload();
    } catch (error) {
      setSwitchError(errorMessage(error));
      setSwitchPendingId(null);
    }
  };

  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>{t('settings:accountKey.title')}</h3>
        <small>{t('settings:accountKey.summary')}</small>
      </CardHeader>

      <Notice>{t('settings:accountKey.scopeNotice')}</Notice>

      <section className='space-y-3'>
        <h4 className='text-sm font-semibold text-foreground'>
          {t('settings:accountKey.accountsTitle')}
        </h4>
        {accountsError ? <Notice tone='destructive'>{accountsError}</Notice> : null}
        {switchError ? <Notice tone='destructive'>{switchError}</Notice> : null}
        <ul className='space-y-2' data-testid='account-list'>
          {(accounts?.accounts ?? []).map((account) => {
            const active = account.id === accounts?.active_account_id;
            return (
              <li
                key={account.id}
                className='flex min-w-0 items-center gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3'
              >
                <div className='min-w-0 flex-1'>
                  <p className='truncate text-sm text-foreground'>
                    {account.label ?? t('settings:accountKey.unnamedAccount')}
                    {active ? (
                      <span className='ml-2 rounded bg-[var(--surface-accent-soft)] px-2 py-0.5 text-xs text-[var(--accent-foreground)]'>
                        {t('settings:accountKey.activeBadge')}
                      </span>
                    ) : null}
                  </p>
                  <p className='break-all font-mono text-xs text-[var(--muted-foreground)]'>
                    {account.pubkey}
                  </p>
                </div>
                {active ? null : (
                  <Button
                    variant='secondary'
                    disabled={switchPendingId !== null}
                    onClick={() => void handleSwitch(account.id)}
                    data-testid={`switch-account-${account.id}`}
                  >
                    {switchPendingId === account.id
                      ? t('settings:accountKey.switchPending')
                      : t('settings:accountKey.switchButton')}
                  </Button>
                )}
              </li>
            );
          })}
        </ul>
      </section>

      <section className='space-y-3'>
        <h4 className='text-sm font-semibold text-foreground'>
          {t('settings:accountKey.export.title')}
        </h4>
        <Notice tone='destructive'>{t('settings:accountKey.export.warning')}</Notice>
        <label className='flex min-w-0 items-center gap-3 text-sm text-foreground'>
          <input
            type='checkbox'
            checked={exportAcknowledged}
            onChange={(event) => setExportAcknowledged(event.currentTarget.checked)}
            data-testid='export-acknowledge'
          />
          <span>{t('settings:accountKey.export.acknowledge')}</span>
        </label>
        <Field
          label={t('settings:accountKey.export.passphraseLabel')}
          hint={t('settings:accountKey.export.passphraseHint', {
            min: MIN_PASSPHRASE_CHARS,
          })}
          message={passphraseTooShort ? t('settings:accountKey.export.tooShort') : undefined}
          tone={passphraseTooShort ? 'danger' : 'default'}
        >
          <Input
            type='password'
            value={exportPassphrase}
            disabled={!exportAcknowledged}
            onChange={(event) => setExportPassphrase(event.currentTarget.value)}
            data-testid='export-passphrase'
          />
        </Field>
        <Field
          label={t('settings:accountKey.export.confirmLabel')}
          message={passphraseMismatch ? t('settings:accountKey.export.mismatch') : undefined}
          tone={passphraseMismatch ? 'danger' : 'default'}
        >
          <Input
            type='password'
            value={exportPassphraseConfirm}
            disabled={!exportAcknowledged}
            onChange={(event) => setExportPassphraseConfirm(event.currentTarget.value)}
            data-testid='export-passphrase-confirm'
          />
        </Field>
        <Button disabled={!exportReady} onClick={() => void handleExport()} data-testid='export-submit'>
          {exportPending
            ? t('settings:accountKey.export.pending')
            : t('settings:accountKey.export.submit')}
        </Button>
        {exportError ? <Notice tone='destructive'>{exportError}</Notice> : null}
        {exportResult ? (
          <div className='space-y-2' data-testid='export-result'>
            <Field label={t('settings:accountKey.export.fingerprintLabel')}>
              <p className='break-all font-mono text-xs text-[var(--muted-foreground)]'>
                {exportResult.public_key}
              </p>
            </Field>
            <Field label={t('settings:accountKey.export.resultLabel')}>
              <Textarea
                readOnly
                rows={4}
                value={exportResult.export}
                data-testid='export-envelope'
              />
            </Field>
            <Button variant='secondary' onClick={() => void handleCopyExport()}>
              {exportCopied
                ? t('settings:accountKey.export.copied')
                : t('settings:accountKey.export.copy')}
            </Button>
          </div>
        ) : null}
      </section>

      <section className='space-y-3'>
        <h4 className='text-sm font-semibold text-foreground'>
          {t('settings:accountKey.import.title')}
        </h4>
        <Field label={t('settings:accountKey.import.inputLabel')}>
          <Textarea
            rows={4}
            value={importText}
            onChange={(event) => {
              setImportText(event.currentTarget.value);
              setImportPreview(null);
              setImportResult(null);
            }}
            data-testid='import-input'
          />
        </Field>
        <Button
          variant='secondary'
          disabled={importText.trim().length === 0}
          onClick={() => void handlePreviewImport()}
          data-testid='import-preview-button'
        >
          {t('settings:accountKey.import.previewButton')}
        </Button>
        {importPreview ? (
          <div className='space-y-3' data-testid='import-preview'>
            <Field label={t('settings:accountKey.import.fingerprintLabel')}>
              <p className='break-all font-mono text-xs text-[var(--muted-foreground)]'>
                {importPreview.public_key}
              </p>
            </Field>
            {importPreview.already_registered ? (
              <Notice tone='destructive'>
                {t('settings:accountKey.import.alreadyRegistered')}
              </Notice>
            ) : null}
            <Field label={t('settings:accountKey.import.passphraseLabel')}>
              <Input
                type='password'
                value={importPassphrase}
                onChange={(event) => setImportPassphrase(event.currentTarget.value)}
                data-testid='import-passphrase'
              />
            </Field>
            <Field label={t('settings:accountKey.import.labelLabel')}>
              <Input
                value={importLabel}
                onChange={(event) => setImportLabel(event.currentTarget.value)}
                data-testid='import-label'
              />
            </Field>
            <Button
              disabled={
                importPending ||
                importPassphrase.length === 0 ||
                importPreview.already_registered
              }
              onClick={() => void handleImport()}
              data-testid='import-submit'
            >
              {importPending
                ? t('settings:accountKey.import.pending')
                : t('settings:accountKey.import.submit')}
            </Button>
          </div>
        ) : null}
        {importError ? <Notice tone='destructive'>{importError}</Notice> : null}
        {importResult ? (
          <Notice tone='accent' data-testid='import-success'>
            {t('settings:accountKey.import.success')}
            <Button
              className='ml-3'
              variant='secondary'
              disabled={switchPendingId !== null}
              onClick={() => void handleSwitch(importResult.id)}
              data-testid='import-switch-now'
            >
              {t('settings:accountKey.import.switchNow')}
            </Button>
          </Notice>
        ) : null}
      </section>
    </Card>
  );
}
