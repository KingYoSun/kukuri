import { cn } from '@/lib/utils';

type RelationshipBadgeProps = {
  label: string | null;
  className?: string;
};

export function RelationshipBadge({ label, className }: RelationshipBadgeProps) {
  if (!label) {
    return null;
  }

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
      {label}
    </span>
  );
}
