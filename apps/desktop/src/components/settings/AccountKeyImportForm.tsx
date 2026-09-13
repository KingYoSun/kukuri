import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { AccountKeyImportPreview, AccountRecord } from '@/lib/api/types.generated';
import { importAccountKey, previewAccountKeyImport } from '@/lib/api/identity';
import { Button } from '@/components/ui/button';
import { Field } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Notice } from '@/components/ui/notice';
import { Textarea } from '@/components/ui/textarea';

const errorMessage = (error: unknown) => error instanceof Error ? error.message : String(error);

export function AccountKeyImportForm({ onImported, onSwitch, switching = false }: {
  onImported: () => Promise<void>;
  onSwitch: (id: string) => void | Promise<void>;
  switching?: boolean;
}) {
  const { t } = useTranslation('settings');
  const [importText, setImportText] = useState('');
  const [importPreview, setImportPreview] = useState<AccountKeyImportPreview | null>(null);
  const [importPassphrase, setImportPassphrase] = useState('');
  const [importLabel, setImportLabel] = useState('');
  const [importPending, setImportPending] = useState(false);
  const [importResult, setImportResult] = useState<AccountRecord | null>(null);
  const [importError, setImportError] = useState<string | null>(null);

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
      await onImported();
    } catch (error) {
      setImportError(errorMessage(error));
    } finally {
      setImportPending(false);
    }
  };

  return (
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
              disabled={switching}
              onClick={() => void onSwitch(importResult.id)}
              data-testid='import-switch-now'
            >
              {t('settings:accountKey.import.switchNow')}
            </Button>
          </Notice>
        ) : null}
      </section>
  );
}
