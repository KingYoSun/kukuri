import { cn } from '@/lib/utils';

type AuthorAvatarProps = {
  label: string;
  picture?: string | null;
  size?: 'sm' | 'lg';
  className?: string;
  testId?: string;
};

export function AuthorAvatar({
  label,
  picture = null,
  size = 'sm',
  className,
  testId,
}: AuthorAvatarProps) {
  const trimmedLabel = label.trim();
  const placeholder = trimmedLabel.charAt(0).toUpperCase() || '#';

  return (
    <div
      className={cn(
        'author-avatar',
        size === 'lg' ? 'author-avatar-lg' : 'author-avatar-sm',
        className
      )}
      data-testid={testId}
      data-avatar-src={picture ?? undefined}
      aria-hidden='true'
    >
      {picture ? (
        <img src={picture} alt='' className='author-avatar-image' />
      ) : (
        <span className='author-avatar-placeholder'>{placeholder}</span>
      )}
    </div>
  );
}
