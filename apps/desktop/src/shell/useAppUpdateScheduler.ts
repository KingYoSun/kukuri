import { useEffect } from 'react';

import {
  DESKTOP_DISTRIBUTION,
  type DesktopDistribution,
  usesSelfManagedUpdater,
} from '@/lib/distribution';

const UPDATE_CHECK_INTERVAL_MS = 30 * 60 * 1000;

export function useAppUpdateScheduler(
  checkForUpdate: () => Promise<void>,
  distribution: DesktopDistribution = DESKTOP_DISTRIBUTION
) {
  useEffect(() => {
    if (
      typeof window === 'undefined' ||
      !('__TAURI_INTERNALS__' in window) ||
      !usesSelfManagedUpdater(distribution)
    ) return;
    void checkForUpdate();
    const intervalId = window.setInterval(() => void checkForUpdate(), UPDATE_CHECK_INTERVAL_MS);
    return () => window.clearInterval(intervalId);
  }, [checkForUpdate, distribution]);
}
