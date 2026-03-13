import { ReactNode } from 'react';
import { useTranslation } from 'react-i18next';

import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Button } from '@/components/ui/button';
import { Textarea } from '@/components/ui/textarea';
import { Loader2, X } from 'lucide-react';

interface PostActionComposerProps {
  avatarSrc: string;
  initials: string;
  content: string;
  placeholder: string;
  autoFocus?: boolean;
  isPending: boolean;
  submitLabel: string;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
  onContentChange: (value: string) => void;
  onShortcut: (event: React.KeyboardEvent<HTMLTextAreaElement>) => void;
  onCancel?: () => void;
  hintText?: string;
  children?: ReactNode;
  dataTestId?: string;
  submitDataTestId?: string;
}

export const PostActionComposer = ({
  avatarSrc,
  initials,
  content,
  placeholder,
  autoFocus = true,
  isPending,
  submitLabel,
  onSubmit,
  onContentChange,
  onShortcut,
  onCancel,
  hintText,
  children,
  dataTestId,
  submitDataTestId,
}: PostActionComposerProps) => {
  const { t } = useTranslation();
  const defaultHintText = hintText ?? t('posts.composer.hint');
  return (
    <form onSubmit={onSubmit} className="space-y-3">
      <div className="flex gap-3">
        <Avatar className="h-8 w-8">
          <AvatarImage src={avatarSrc} />
          <AvatarFallback>{initials}</AvatarFallback>
        </Avatar>
        <div className="flex-1 space-y-3">
          <Textarea
            value={content}
            onChange={(event) => onContentChange(event.target.value)}
            onKeyDown={onShortcut}
            placeholder={placeholder}
            className="min-h-[80px] resize-none"
            autoFocus={autoFocus}
            disabled={isPending}
            data-testid={dataTestId}
          />
          {children}
          <div className="flex items-center justify-between">
            <p className="text-xs text-muted-foreground">{defaultHintText}</p>
            <div className="flex gap-2">
              {onCancel && (
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={onCancel}
                  disabled={isPending}
                >
                  <X className="mr-1 h-4 w-4" />
                  {t('common.cancel')}
                </Button>
              )}
              <Button
                type="submit"
                size="sm"
                disabled={!content.trim() || isPending}
                data-testid={submitDataTestId}
              >
                {isPending ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    {t('posts.composer.posting')}
                  </>
                ) : (
                  submitLabel
                )}
              </Button>
            </div>
          </div>
        </div>
      </div>
    </form>
  );
};
