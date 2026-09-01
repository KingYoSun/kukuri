import type {
  AppConsentStatus,
  DesktopStartupStatus,
} from './types';

import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// DesktopApi 外のスタンドアロンコマンド。mock ビルドでは window.__KUKURI_DESKTOP__ に
// 同名メソッドが無いため、runtimeApi の command() ラッパーではなく固定スタブを返す
// (分岐の判定だけ isDesktopMockActive() へ集約している)。
export async function getAppConsentStatus(): Promise<AppConsentStatus> {
  if (isDesktopMockActive()) {
    return {
      documents: [
        {
          slug: 'terms',
          currentVersion: 1,
          acceptedVersion: 1,
          acceptedAt: null,
          acceptedLanguage: null,
          acceptedAppVersion: null,
        },
        {
          slug: 'privacy',
          currentVersion: 1,
          acceptedVersion: 1,
          acceptedAt: null,
          acceptedLanguage: null,
          acceptedAppVersion: null,
        },
      ],
      satisfied: true,
    };
  }
  return invokeDesktop<AppConsentStatus>('get_app_consent_status');
}

export type AcceptedAppConsentDocument = {
  slug: string;
  version: number;
};

export async function acceptAppConsents(
  documents: AcceptedAppConsentDocument[],
  language: string
): Promise<DesktopStartupStatus> {
  if (isDesktopMockActive()) {
    return { status: 'ready' };
  }
  return invokeDesktop<DesktopStartupStatus>('accept_app_consents', {
    documents,
    language,
  });
}
