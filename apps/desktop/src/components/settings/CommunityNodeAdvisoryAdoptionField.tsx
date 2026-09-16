import { useId } from 'react';
import { useTranslation } from 'react-i18next';

/// #1056: Community Node ごとに、そのノードの成人向け表現の推定(content advisory)を採用するかを選ぶ。
///
/// 採用すると、成人向け表現を非表示にしている間、このノードが推定した投稿も代替表示にする。そのため
/// 表示中の投稿 ID と添付の識別子をこのノードへ照会する。採用しないノードへは照会しない
/// (ADR 0046 §6.1 の node 単位の opt-in)。変更は既存の「ノードを保存」で確定する。
export type CommunityNodeAdvisoryAdoptionFieldProps = {
  nodeId: string;
  enabled: boolean;
  disabled?: boolean;
  onChange: (enabled: boolean) => void;
};

export function CommunityNodeAdvisoryAdoptionField({
  nodeId,
  enabled,
  disabled = false,
  onChange,
}: CommunityNodeAdvisoryAdoptionFieldProps) {
  const { t } = useTranslation(['settings']);
  const descriptionId = useId();

  return (
    <section
      className='mt-4 space-y-3 rounded-[16px] border border-[var(--border-subtle)] p-4'
      data-testid={`community-node-advisory-adoption-${nodeId}`}
    >
      <label className='flex min-w-0 items-start gap-3 text-sm font-medium text-foreground'>
        <input
          type='checkbox'
          className='mt-0.5 size-4 shrink-0'
          checked={enabled}
          disabled={disabled}
          aria-describedby={descriptionId}
          data-testid={`community-node-advisory-toggle-${nodeId}`}
          onChange={(event) => onChange(event.currentTarget.checked)}
        />
        <span className='min-w-0'>{t('settings:communityNode.contentAdvisory.label')}</span>
      </label>
      <div id={descriptionId} className='space-y-1'>
        <p className='text-sm text-[var(--muted-foreground)]'>
          {t('settings:communityNode.contentAdvisory.description')}
        </p>
        <p className='text-sm text-[var(--muted-foreground)]'>
          {enabled
            ? t('settings:communityNode.contentAdvisory.enabled')
            : t('settings:communityNode.contentAdvisory.disabled')}
        </p>
      </div>
    </section>
  );
}
