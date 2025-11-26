import { useEffect, useMemo, useState } from 'react';
import { AlertCircle, Clock } from 'lucide-react';

import { Card, CardContent } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import type { UserSearchErrorKey } from '@/hooks/useUserSearchQuery';

interface SearchErrorStateProps {
  errorKey: UserSearchErrorKey;
  retryAfterSeconds?: number | null;
  onRetry?: () => void;
  onCooldownComplete?: () => void;
}

const ERROR_MESSAGES: Record<
  UserSearchErrorKey,
  { title: string; description: string; showRetry: boolean }
> = {
  'UserSearch.invalid_query': {
    title: '検索キーワードが短すぎます',
    description: '2文字以上入力してください',
    showRetry: false,
  },
  'UserSearch.fetch_failed': {
    title: 'ユーザー検索に失敗しました',
    description: 'ネットワーク状況を確認して、再試行してください',
    showRetry: true,
  },
  'UserSearch.rate_limited': {
    title: 'リクエストが多すぎます',
    description: '一定時間後に再試行してください',
    showRetry: true,
  },
};

export function SearchErrorState({
  errorKey,
  retryAfterSeconds = null,
  onRetry,
  onCooldownComplete,
}: SearchErrorStateProps) {
  const [remainingSeconds, setRemainingSeconds] = useState<number | null>(retryAfterSeconds);

  useEffect(() => {
    setRemainingSeconds(retryAfterSeconds ?? null);
  }, [retryAfterSeconds]);

  useEffect(() => {
    if (remainingSeconds === null) {
      return;
    }
    if (remainingSeconds <= 0) {
      onCooldownComplete?.();
      return;
    }
    const timer = setTimeout(() => {
      setRemainingSeconds((value) => (value === null ? null : Math.max(value - 1, 0)));
    }, 1000);
    return () => clearTimeout(timer);
  }, [remainingSeconds, onCooldownComplete]);

  const { title, description, showRetry } = ERROR_MESSAGES[errorKey];

  const retryLabel = useMemo(() => {
    if (errorKey !== 'UserSearch.rate_limited' || remainingSeconds === null) {
      return '再試行';
    }
    if (remainingSeconds <= 0) {
      return '再試行';
    }
    return `再試行 (${remainingSeconds}s)`;
  }, [errorKey, remainingSeconds]);

  const isRetryDisabled = errorKey === 'UserSearch.rate_limited' && (remainingSeconds ?? 0) > 0;

  return (
    <Card className="border-dashed" data-testid="user-search-error">
      <CardContent className="flex items-center gap-3 py-6">
        {errorKey === 'UserSearch.rate_limited' ? (
          <Clock className="h-5 w-5 text-yellow-500" />
        ) : (
          <AlertCircle className="h-5 w-5 text-destructive" />
        )}
        <div className="flex-1 space-y-1">
          <p className="font-medium">{title}</p>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>
        {showRetry && onRetry && (
          <Button
            variant="outline"
            onClick={onRetry}
            disabled={isRetryDisabled}
            data-testid="user-search-retry-button"
            aria-disabled={isRetryDisabled}
          >
            {retryLabel}
          </Button>
        )}
      </CardContent>
    </Card>
  );
}
