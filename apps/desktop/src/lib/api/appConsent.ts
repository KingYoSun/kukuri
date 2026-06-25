import type { AppConsentStatus, DesktopStartupStatus } from './types';

import { invokeDesktop } from './invoke/desktop';

export async function getAppConsentStatus(): Promise<AppConsentStatus> {
  if (window.__KUKURI_DESKTOP__) {
    return {
      currentBundleVersion: 1,
      acceptedBundleVersion: 1,
      acceptedAt: null,
      satisfied: true,
    };
  }
  return invokeDesktop<AppConsentStatus>('get_app_consent_status');
}

export async function acceptAppConsents(
  bundleVersion: number
): Promise<DesktopStartupStatus> {
  if (window.__KUKURI_DESKTOP__) {
    return { status: 'ready' };
  }
  return invokeDesktop<DesktopStartupStatus>('accept_app_consents', {
    bundleVersion,
  });
}
