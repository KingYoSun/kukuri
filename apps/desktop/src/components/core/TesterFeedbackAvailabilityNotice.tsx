import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import type { TesterFeedbackAvailability } from '@/lib/api/testerFeedbackAvailability';

export function TesterFeedbackAvailabilityNotice({ availability, onOpenSettings, cachedNodes = false }: {
  availability: TesterFeedbackAvailability;
  onOpenSettings: () => void;
  cachedNodes?: boolean;
}) {
  const { t } = useTranslation('shell');
  return (
    <Notice>
      <div className='extended-module-stack'>
        <p role='status'>{t(`testerFeedback.availability.${availability.state}`)}</p>
        {cachedNodes ? <p>{t('testerFeedback.cachedNodesHint')}</p> : null}
        {availability.nodes.length > 0 ? (
          <ul className='extended-module-stack'>
            {availability.nodes.map(node => (
              <li key={node.baseUrl}>
                <strong className='break-words [overflow-wrap:anywhere]'>{node.label}</strong>
                <p>{t(`testerFeedback.reasons.${node.reason}`)}</p>
              </li>
            ))}
          </ul>
        ) : null}
        <p>{t('testerFeedback.intakeRequirement')}</p>
        <Button type='button' variant='secondary' onClick={onOpenSettings}>
          {t('testerFeedback.openSettings')}
        </Button>
        <p className='muted'>{t('testerFeedback.settingsReturnHint')}</p>
      </div>
    </Notice>
  );
}
