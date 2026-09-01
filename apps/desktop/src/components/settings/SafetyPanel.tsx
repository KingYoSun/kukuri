import { useTranslation } from 'react-i18next';

import { Card, CardHeader } from '@/components/ui/card';
import { Notice } from '@/components/ui/notice';

type SafetyPanelProps = {
  adultContentEnabled: boolean;
  onAdultContentEnabledChange: (enabled: boolean) => void;
};

// #858: 成人向け表現の表示設定(ADR 0046)。既定 OFF で、18歳以上の自己申告とは
// 別の状態。ON にしない限り、対象メディアのバイト列は取得もデコードもされない。
export function SafetyPanel({
  adultContentEnabled,
  onAdultContentEnabledChange,
}: SafetyPanelProps) {
  const { t } = useTranslation(['settings']);

  return (
    <Card className='space-y-4'>
      <CardHeader>
        <h3>{t('settings:safety.title')}</h3>
        <small>{t('settings:safety.summary')}</small>
      </CardHeader>

      <label className='flex min-w-0 items-center gap-3 rounded-[var(--radius-input)] border border-[var(--border-subtle)] bg-[var(--surface-panel-soft)] px-4 py-3 text-sm text-foreground'>
        <input
          type='checkbox'
          checked={adultContentEnabled}
          onChange={(event) => onAdultContentEnabledChange(event.currentTarget.checked)}
          data-testid='adult-content-display-toggle'
        />
        <span>{t('settings:safety.adultContent.label')}</span>
      </label>

      <Notice>{t('settings:safety.adultContent.description')}</Notice>
    </Card>
  );
}
