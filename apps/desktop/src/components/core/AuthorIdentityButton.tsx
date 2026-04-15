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
    <button
      className={cn('post-meta-author author-link', className, buttonClassName)}
      type='button'
      onClick={onClick}
    >
      <AuthorAvatar
        label={label}
        picture={picture}
        size={avatarSize}
        testId={avatarTestId}
      />
      <span>{label}</span>
    </button>
  );
}
