import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';

/// #1061: 信頼値による表示判断で採用するノードの優先順位（ADR 0026 §8.4）。
///
/// 上位から順に、有効な評価を返したノードの判断を採る。選ばなければ、この機能による
/// 折りたたみは行わない。異なるノードの評価を混ぜたり平均したりはしない。
export type CommunityNodeTrustPriorityFieldProps = {
  /// 設定済みノードの base URL（表示順）。
  configuredBaseUrls: readonly string[];
  /// 採用順位（上位から）。
  priority: readonly string[];
  disabled?: boolean;
  onChange: (priority: string[]) => void;
};

export function CommunityNodeTrustPriorityField({
  configuredBaseUrls,
  priority,
  disabled = false,
  onChange,
}: CommunityNodeTrustPriorityFieldProps) {
  const { t } = useTranslation(['common', 'settings']);
  const descriptionId = useId();
  const selected = priority.filter((baseUrl) => configuredBaseUrls.includes(baseUrl));
  const move = (index: number, delta: number) => {
    const next = [...selected];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  return (
    <section
      className='mt-4 space-y-3 rounded-[16px] border border-[var(--border-subtle)] p-4'
      data-testid='community-node-trust-priority'
    >
      <div className='space-y-1'>
        <h5 className='text-sm font-semibold text-foreground'>
          {t('settings:communityNode.trustPriority.title')}
        </h5>
        <p id={descriptionId} className='text-sm text-[var(--muted-foreground)]'>
          {t('settings:communityNode.trustPriority.description')}
        </p>
        {selected.length === 0 ? (
          <p className='text-sm text-[var(--muted-foreground)]'>
            {t('settings:communityNode.trustPriority.none')}
          </p>
        ) : null}
      </div>
      <ul className='space-y-2'>
        {configuredBaseUrls.map((baseUrl) => {
          const index = selected.indexOf(baseUrl);
          const adopted = index >= 0;
          return (
            <li key={baseUrl} className='flex flex-wrap items-center gap-2 text-sm'>
              <label className='flex min-w-0 flex-1 items-center gap-2'>
                <input
                  type='checkbox'
                  className='size-4 shrink-0'
                  checked={adopted}
                  disabled={disabled}
                  aria-describedby={descriptionId}
                  data-testid={`community-node-trust-priority-toggle-${baseUrl}`}
                  onChange={(event) =>
                    onChange(
                      event.currentTarget.checked
                        ? [...selected, baseUrl]
                        : selected.filter((value) => value !== baseUrl)
                    )
                  }
                />
                <span className='min-w-0 break-all font-mono text-xs'>{baseUrl}</span>
              </label>
              {adopted ? (
                <span className='flex items-center gap-2'>
                  <span className='text-xs text-[var(--muted-foreground)]'>
                    {t('settings:communityNode.trustPriority.rank', { rank: index + 1 })}
                  </span>
                  <Button
                    variant='secondary'
                    disabled={disabled || index === 0}
                    aria-label={t('settings:communityNode.trustPriority.moveUp', { baseUrl })}
                    onClick={() => move(index, -1)}
                  >
                    ↑
                  </Button>
                  <Button
                    variant='secondary'
                    disabled={disabled || index === selected.length - 1}
                    aria-label={t('settings:communityNode.trustPriority.moveDown', { baseUrl })}
                    onClick={() => move(index, 1)}
                  >
                    ↓
                  </Button>
                </span>
              ) : null}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
