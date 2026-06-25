import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { LegalDocumentView } from '@/components/LegalDocumentView';
import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';
import { getAppConsentStatus, type AppConsentStatus } from '@/lib/api';
import { getResolvedLocale } from '@/i18n/format';
import { useAppUpdateStore } from '@/shell/useAppUpdateStore';

import { SettingsDiagnosticList } from './SettingsDiagnosticList';

function formatAcceptedAt(value: number | null | undefined, locale: string): string | null {
  if (!value) {
    return null;
  }
  return new Intl.DateTimeFormat(getResolvedLocale(locale), {
    dateStyle: 'medium',
    timeStyle: 'medium',
  }).format(new Date(value * 1000));
}

export function AboutPanel() {
  const { t, i18n } = useTranslation(['legal', 'common']);
  const currentVersion = useAppUpdateStore((state) => state.updateState.currentVersion);
  const [consentStatus, setConsentStatus] = useState<AppConsentStatus | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let disposed = false;
    void getAppConsentStatus()
      .then((status) => {
        if (!disposed) {
          setConsentStatus(status);
          setError(null);
        }
      })
      .catch((caught) => {
        if (!disposed) {
          setError(caught instanceof Error ? caught.message : String(caught));
        }
      });
    return () => {
      disposed = true;
    };
  }, []);

  const acceptedAt = formatAcceptedAt(consentStatus?.acceptedAt, i18n.resolvedLanguage ?? i18n.language);
  const diagnostics = useMemo(
    () => [
      {
        label: t('legal:about.appVersionLabel'),
        value: currentVersion,
        monospace: true,
      },
      {
        label: t('legal:about.bundleVersionLabel'),
        value: String(consentStatus?.currentBundleVersion ?? 1),
      },
      {
        label: t('legal:about.acceptedVersionLabel'),
        value: consentStatus?.acceptedBundleVersion
          ? String(consentStatus.acceptedBundleVersion)
          : t('legal:about.notAccepted'),
      },
      {
        label: t('legal:about.acceptedAtLabel'),
        value: acceptedAt ?? t('legal:about.notAccepted'),
      },
    ],
    [acceptedAt, consentStatus, currentVersion, t]
  );

  return (
    <Card className='min-w-0 space-y-5'>
      <CardHeader>
        <h3>{t('legal:about.title')}</h3>
        <small>{t('legal:about.summary')}</small>
      </CardHeader>
      {error ? <Notice tone='destructive'>{error}</Notice> : null}
      <SettingsDiagnosticList items={diagnostics} columns={2} />
      <section className='min-w-0 space-y-4'>
        <h4 className='text-base font-semibold text-foreground'>
          {t('legal:about.documentsHeading')}
        </h4>
        <LegalDocumentView bundleVersion={consentStatus?.currentBundleVersion ?? 1} compact />
      </section>
    </Card>
  );
}
