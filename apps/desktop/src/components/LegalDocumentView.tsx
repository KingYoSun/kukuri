import { useTranslation } from 'react-i18next';

import { Notice } from '@/components/ui/notice';

export type LegalDocumentKind = 'terms' | 'privacy';

type LegalDocumentSection = {
  heading: string;
  paragraphs: string[];
};

type LegalDocumentViewProps = {
  bundleVersion?: number | null;
  compact?: boolean;
};

const DOCUMENT_KINDS: LegalDocumentKind[] = ['terms', 'privacy'];

export function LegalDocumentView({ bundleVersion, compact = false }: LegalDocumentViewProps) {
  const { t } = useTranslation('legal');

  return (
    <div className={compact ? 'space-y-5' : 'space-y-6'}>
      <Notice>{t('documents.draftNotice')}</Notice>
      {bundleVersion ? (
        <p className='text-xs font-semibold uppercase tracking-[0.08em] text-[var(--muted-foreground)]'>
          v{bundleVersion}
        </p>
      ) : null}
      {DOCUMENT_KINDS.map((kind) => {
        const sections = t(`documents.${kind}.sections`, {
          returnObjects: true,
        }) as LegalDocumentSection[];
        return (
          <article key={kind} className={compact ? 'space-y-3' : 'space-y-4'}>
            <h3 className='text-lg font-semibold text-foreground'>
              {t(`documents.${kind}.title`)}
            </h3>
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
