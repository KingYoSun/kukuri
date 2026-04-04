import { cn } from '@/lib/utils';

import { AuthorAvatar } from './AuthorAvatar';

type AuthorIdentityButtonProps = {
  label: string;
  picture?: string | null;
  onClick: () => void;
  avatarSize?: 'sm' | 'lg';
  avatarTestId?: string;
  className?: string;
  buttonClassName?: string;
};

export function AuthorIdentityButton({
  label,
  picture = null,
  onClick,
  avatarSize = 'sm',
  avatarTestId,
  className,
  buttonClassName,
}: AuthorIdentityButtonProps) {
  return (
    <div className={cn('post-meta-author', className)}>
      <AuthorAvatar
        label={label}
        picture={picture}
        size={avatarSize}
        testId={avatarTestId}
      />
      <button
        className={cn('author-link', buttonClassName)}
        type='button'
        onClick={onClick}
      >
        {label}
      </button>
    </div>
  );
}
