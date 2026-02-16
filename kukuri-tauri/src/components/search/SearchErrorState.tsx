import { useTranslation } from 'react-i18next';
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

export function SearchErrorState({
  errorKey,
  retryAfterSeconds = null,
  onRetry,
  onCooldownComplete,
}: SearchErrorStateProps) {
  const { t } = useTranslation();
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

  const title = t(`search.error.${errorKey}.title`);
  const description = t(`search.error.${errorKey}.description`);
  const showRetry = errorKey !== 'UserSearch.invalid_query';

  const retryLabel = useMemo(() => {
    if (errorKey !== 'UserSearch.rate_limited' || remainingSeconds === null) {
      return t('search.error.retry');
    }
    if (remainingSeconds <= 0) {
      return t('search.error.retry');
    }
    return t('search.error.retryWithSeconds', { seconds: remainingSeconds });
  }, [errorKey, remainingSeconds, t]);

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
