import { useTranslation } from 'react-i18next';

export type LegalDocumentKind = 'terms' | 'privacy';

export type LegalDocumentMetadata = {
  effectiveDate: string;
  authoritativeLanguage: string;
  materialChange: boolean;
  controllerName: string;
  contact: string;
};

type LegalDocumentSection = {
  heading: string;
  paragraphs: string[];
};

type LegalDocumentViewProps = {
  /** #857: 文書単位の版表示。slug -> 現行版。 */
  documentVersions?: Partial<Record<LegalDocumentKind, number>> | null;
  documentMetadata?: Partial<Record<LegalDocumentKind, LegalDocumentMetadata>> | null;
  compact?: boolean;
};

const DOCUMENT_KINDS: LegalDocumentKind[] = ['terms', 'privacy'];

export function LegalDocumentView({
  documentVersions,
  documentMetadata,
  compact = false,
}: LegalDocumentViewProps) {
  const { t, i18n } = useTranslation('legal');
  const displayLanguage = i18n.resolvedLanguage ?? i18n.language;

  return (
    <div className={compact ? 'space-y-5' : 'space-y-6'}>
      {DOCUMENT_KINDS.map((kind) => {
        const sections = t(`documents.${kind}.sections`, {
          returnObjects: true,
          controllerName: documentMetadata?.[kind]?.controllerName,
          contact: documentMetadata?.[kind]?.contact,
        }) as LegalDocumentSection[];
        const version = documentVersions?.[kind];
        const metadata = documentMetadata?.[kind];
        const referenceTranslation = metadata
          ? !displayLanguage.toLowerCase().startsWith(
              metadata.authoritativeLanguage.toLowerCase()
            )
          : false;
        return (
          <article key={kind} className={compact ? 'space-y-3' : 'space-y-4'}>
            <h3 className='text-lg font-semibold text-foreground'>
              {t(`documents.${kind}.title`)}
              {version ? (
                <span className='ml-2 text-xs font-semibold uppercase tracking-[0.08em] text-[var(--muted-foreground)]'>
                  v{version}
                </span>
              ) : null}
            </h3>
            {metadata ? (
              <div className='space-y-1 text-xs leading-5 text-[var(--muted-foreground-soft)]'>
                <p>
                  {t('metadata.effectiveDate', { date: metadata.effectiveDate })}
                  {' · '}
                  {t('metadata.authoritativeLanguage')}
                </p>
                {referenceTranslation ? <p>{t('metadata.referenceTranslation')}</p> : null}
                {metadata.materialChange ? (
                  <p>{t(`documents.${kind}.changeSummary`)}</p>
                ) : null}
              </div>
            ) : null}
            <div className={compact ? 'space-y-3' : 'space-y-4'}>
              {sections.map((section) => (
                <section key={section.heading} className='space-y-2'>
                  <h4 className='text-base font-semibold text-foreground'>{section.heading}</h4>
                  {section.paragraphs.map((paragraph) => (
                    <p key={paragraph} className='text-sm leading-6 text-[var(--muted-foreground)]'>
                      {paragraph}
                    </p>
                  ))}
                </section>
              ))}
            </div>
          </article>
        );
      })}
    </div>
  );
}
