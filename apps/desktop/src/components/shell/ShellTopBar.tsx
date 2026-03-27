import * as React from 'react';
import { useTranslation } from 'react-i18next';

type ShellTopBarProps = {
  activeTopic: string;
};

export function ShellTopBar({ activeTopic }: ShellTopBarProps) {
  const { t } = useTranslation('shell');

  return (
    <header className='shell-topbar panel panel-accent' aria-label={t('topBar.activeTopicBar')}>
      <p className='shell-topbar-topic' title={activeTopic}>
        {activeTopic}
      </p>
    </header>
  );
}
