import { useId, useRef, type KeyboardEvent, type ReactNode } from 'react';
import { Box, Bug, CircleUserRound, House, Network, Server, X, Grid2X2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import type { SupportedLocale } from '@/i18n';

const METAVERSE_CATEGORIES = ['dome', 'hosting', 'connections', 'avatar', 'objects', 'diagnostics'] as const;
export type MetaverseCategory = typeof METAVERSE_CATEGORIES[number];
export type MetaverseOverlay = 'closed' | 'chat' | 'categories' | 'details';
export type MetaversePanels = Partial<Record<MetaverseCategory, ReactNode>>;
const icons = [House, Server, Network, CircleUserRound, Box, Bug];

function navigate(event: KeyboardEvent, index: number, buttons: (HTMLButtonElement | null)[], select?: (value: MetaverseCategory) => void) {
  let next: number;
  switch (event.key) {
    case 'ArrowRight': case 'ArrowDown': next = (index + 1) % METAVERSE_CATEGORIES.length; break;
    case 'ArrowLeft': case 'ArrowUp': next = (index + METAVERSE_CATEGORIES.length - 1) % METAVERSE_CATEGORIES.length; break;
    case 'Home': next = 0; break;
    case 'End': next = METAVERSE_CATEGORIES.length - 1; break;
    default: return;
  }
  if (event.nativeEvent.isComposing) return;
  event.preventDefault();
  event.stopPropagation();
  select?.(METAVERSE_CATEGORIES[next]);
  buttons[next]?.focus();
}

export function MetaverseCategories({ overlay, category, locale, panels, onSelect, onCategories, onClose }: {
  overlay: MetaverseOverlay;
  category: MetaverseCategory;
  locale: SupportedLocale;
  panels: MetaversePanels;
  onSelect: (category: MetaverseCategory) => void;
  onCategories: () => void;
  onClose: () => void;
}) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const id = useId();
  const choices = useRef<(HTMLButtonElement | null)[]>([]);
  const tabs = useRef<(HTMLButtonElement | null)[]>([]);
  return <>
    <section className='metaverse-category-menu' hidden={overlay !== 'categories'} aria-label={t('menu.title')}>
      <h3>{t('menu.title')}</h3>
      <div className='metaverse-category-circle'>
        {METAVERSE_CATEGORIES.map((value, index) => {
          const Icon = icons[index];
          return <Button key={value} variant='secondary' type='button' ref={el => { choices.current[index] = el; }}
            data-category={value} data-selected={value === category || undefined}
            onKeyDown={event => navigate(event, index, choices.current)} onClick={() => onSelect(value)}>
            <Icon aria-hidden='true' className='size-5' />{t(`menu.categories.${value}`)}
          </Button>;
        })}
        <Button type='button' variant='ghost' className='metaverse-category-center' onClick={onClose}>
          <X aria-hidden='true' className='size-4' />{t('menu.close')}
        </Button>
      </div>
    </section>
    <aside className='metaverse-room-hud metaverse-category-details' hidden={overlay !== 'details'} aria-label={t('menu.details')}>
      <div className='metaverse-details-header'>
        <Button type='button' variant='ghost' onClick={onCategories}><Grid2X2 className='size-4' aria-hidden='true' />{t('menu.categoriesLabel')}</Button>
        <Button type='button' variant='ghost' onClick={onClose}><X className='size-4' aria-hidden='true' />{t('menu.close')}</Button>
      </div>
      <div role='tablist' aria-label={t('menu.categoriesLabel')} className='metaverse-category-tabs'>
        {METAVERSE_CATEGORIES.map((value, index) => <button key={value} type='button' role='tab'
          id={`${id}-${value}-tab`} aria-controls={`${id}-${value}-panel`} aria-selected={value === category}
          tabIndex={value === category ? 0 : -1} ref={el => { tabs.current[index] = el; }}
          onKeyDown={event => navigate(event, index, tabs.current, onSelect)} onClick={() => onSelect(value)}>
          {t(`menu.categories.${value}`)}
        </button>)}
      </div>
      {METAVERSE_CATEGORIES.map(value => <div key={value} role='tabpanel' tabIndex={0}
        id={`${id}-${value}-panel`} aria-labelledby={`${id}-${value}-tab`} hidden={category !== value}
        className='metaverse-category-content'>
        {panels[value] ?? <p>{t('menu.unavailable')}</p>}
      </div>)}
    </aside>
  </>;
}
