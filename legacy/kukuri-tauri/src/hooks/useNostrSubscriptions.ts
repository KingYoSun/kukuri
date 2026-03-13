import { useCallback, useEffect, useState } from 'react';

import { listNostrSubscriptions, type NostrSubscriptionState } from '@/lib/api/nostr';

interface UseNostrSubscriptionsOptions {
  pollIntervalMs?: number;
}

interface UseNostrSubscriptionsResult {
  subscriptions: NostrSubscriptionState[];
  isLoading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export function useNostrSubscriptions({
  pollIntervalMs = 15000,
}: UseNostrSubscriptionsOptions = {}): UseNostrSubscriptionsResult {
  const [subscriptions, setSubscriptions] = useState<NostrSubscriptionState[]>([]);
  const [isLoading, setIsLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    try {
      const result = await listNostrSubscriptions();
      setSubscriptions(result);
      setError(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();

    if (pollIntervalMs <= 0) {
      return;
    }

    const id = window.setInterval(refresh, pollIntervalMs);
    return () => window.clearInterval(id);
  }, [pollIntervalMs, refresh]);

  return {
    subscriptions,
    isLoading,
    error,
    refresh,
  };
}
