import { useTranslation } from 'react-i18next';

import { cn } from '@/lib/utils';

type RelationshipBadgeProps = {
  label: string | null;
  className?: string;
};

export function RelationshipBadge({ label, className }: RelationshipBadgeProps) {
  const { t } = useTranslation('common');

  if (!label) {
    return null;
  }

  const displayLabel =
    label === 'mutual'
      ? t('relationships.mutual')
      : label === 'friend of friend'
        ? t('relationships.friendOfFriend')
        : label === 'following'
          ? t('relationships.following')
          : label === 'follows you'
            ? t('relationships.followsYou')
            : label;

  return (
    <span
      className={cn(
        'relationship-badge',
        label === 'mutual' && 'relationship-badge-mutual',
        label === 'friend of friend' && 'relationship-badge-fof',
        (label === 'following' || label === 'follows you') && 'relationship-badge-direct',
        className
      )}
    >
      {displayLabel}
    </span>
  );
}
