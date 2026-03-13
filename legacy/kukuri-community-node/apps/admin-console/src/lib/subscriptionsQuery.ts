import { api } from './api';

export const subscriptionsQueryKey = (filter = '') => ['subscriptions', filter] as const;

export const subscriptionsQueryOptions = (filter = '') => ({
  queryKey: subscriptionsQueryKey(filter),
  queryFn: () => api.subscriptions(filter || undefined)
});
