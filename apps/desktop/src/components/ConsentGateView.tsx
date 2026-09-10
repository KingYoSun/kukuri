import { useId } from 'react';
import { useTranslation } from 'react-i18next';
import { LockKeyhole } from 'lucide-react';
import type { AppConsentDocumentStatus } from '@/lib/api';
import { LegalDocumentView } from './LegalDocumentView';
import { LocaleSelect } from './LocaleSelect';
import type { SupportedLocale } from '@/i18n';
import { Button } from './ui/button';
import { Notice } from './ui/notice';

export type ConsentGateViewProps = {
  documents: AppConsentDocumentStatus[];
  updated: boolean;
  attestationRequired: boolean;
  ageAttested: boolean;
  accepting: boolean;
  error: string | null;
  declined: boolean;
  locale: SupportedLocale;
  localeSaveFailed?: boolean;
  onLocaleChange: (locale: SupportedLocale) => void;
  onAgeAttestedChange: (value: boolean) => void;
  onAccept: () => void;
  onDecline: () => void;
};

// 起動・保存の判定はAppのConsentGateが所有する。ここは同じ状態の描画確認面でもある。
export function ConsentGateView({
  documents, updated, attestationRequired, ageAttested, accepting, error, declined,
  locale, localeSaveFailed, onLocaleChange,
  onAgeAttestedChange, onAccept, onDecline,
}: ConsentGateViewProps) {
  const { t } = useTranslation('legal');
  const titleId = useId();
  const reasonId = useId();
  const missingAttestation = attestationRequired && !ageAttested;

  return (
    <main className='startup-error-screen app-consent-screen'>
      <section className='startup-error-panel app-consent-panel' aria-labelledby={titleId}>
        <header className='space-y-2'>
          <h1 id={titleId} className='text-xl font-semibold text-foreground'>{t('gate.title')}</h1>
          <p className='text-sm leading-6 text-[var(--muted-foreground)]'>{t('gate.intro')}</p>
          <LocaleSelect value={locale} onChange={onLocaleChange} disabled={accepting} saveFailed={localeSaveFailed} />
          {updated ? <Notice tone='warning'>{t('gate.updatedNotice')}</Notice> : null}
        </header>
        <div className='app-consent-documents' role='region' aria-label={t('gate.documentsLabel')} tabIndex={0}>
          <LegalDocumentView
            documentVersions={Object.fromEntries(documents.map((document) => [document.slug, document.currentVersion]))}
            documentMetadata={Object.fromEntries(documents.map((document) => [document.slug, {
              effectiveDate: document.effectiveDate,
              authoritativeLanguage: document.authoritativeLanguage,
              materialChange: document.materialChange,
              controllerName: document.controllerName,
              contact: document.contact,
            }]))}
            compact
          />
        </div>
        <footer className='app-consent-footer'>
          {attestationRequired ? (
            <div className='space-y-2'>
              <label className='app-consent-age-label'>
                <input
                  type='checkbox'
                  checked={ageAttested}
                  disabled={accepting}
                  aria-describedby={missingAttestation ? reasonId : undefined}
                  onChange={(event) => onAgeAttestedChange(event.target.checked)}
                  data-testid='age-attestation-checkbox'
                />
                <span>{t('gate.ageAttestationLabel')}</span>
              </label>
              {missingAttestation ? (
                <p id={reasonId} className='app-consent-reason'>{t('gate.ageAttestationRequired')}</p>
              ) : null}
            </div>
          ) : null}
          <div aria-live='polite' aria-atomic='true'>
            {error ? (
              <Notice tone='destructive'>
                <div className='space-y-1'>
                  <p>{t('gate.acceptError')}</p>
                  <small className='font-mono break-words'>{error}</small>
                </div>
              </Notice>
            ) : null}
            {declined ? <Notice tone='destructive'>{t('gate.declineNotice')}</Notice> : null}
          </div>
          <div className='startup-error-actions app-consent-actions'>
            <Button
              type='button'
              disabled={accepting || missingAttestation}
              aria-describedby={missingAttestation ? reasonId : undefined}
              aria-busy={accepting}
              onClick={onAccept}
            >
              {accepting ? t('gate.accepting') : missingAttestation ? (
                <><LockKeyhole className='size-4 shrink-0' aria-hidden='true' />{t('gate.acceptBlocked')}</>
              ) : t('gate.accept')}
            </Button>
            <Button type='button' variant='secondary' disabled={accepting} onClick={onDecline}>
              {t('gate.decline')}
            </Button>
          </div>
        </footer>
      </section>
    </main>
  );
}
