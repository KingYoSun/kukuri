import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// DesktopApi 外のスタンドアロンコマンド(appConsent.ts と同じ方式)。デスクトップの
// OS トーストに実行時権限は無く、旧 tauri-plugin-notification の desktop 実装も常に
// granted を返していたため、mock ビルドでも同じ固定値を返す。
export async function getOsNotificationPermission(): Promise<string> {
  if (isDesktopMockActive()) {
    return 'granted';
  }
  return invokeDesktop<string>('get_os_notification_permission');
}

export async function requestOsNotificationPermission(): Promise<string> {
  if (isDesktopMockActive()) {
    return 'granted';
  }
  return invokeDesktop<string>('request_os_notification_permission');
}
