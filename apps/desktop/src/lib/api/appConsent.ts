import type { AppConsentStatus, DesktopStartupStatus } from './types';

import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// DesktopApi 外のスタンドアロンコマンド。mock ビルドでは window.__KUKURI_DESKTOP__ に
// 同名メソッドが無いため、runtimeApi の command() ラッパーではなく固定スタブを返す
// (分岐の判定だけ isDesktopMockActive() へ集約している)。
export async function getAppConsentStatus(): Promise<AppConsentStatus> {
  if (isDesktopMockActive()) {
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
  if (isDesktopMockActive()) {
    return { status: 'ready' };
  }
  return invokeDesktop<DesktopStartupStatus>('accept_app_consents', {
    bundleVersion,
  });
}
