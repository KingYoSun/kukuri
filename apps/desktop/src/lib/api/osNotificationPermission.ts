import { invokeDesktop } from './invoke/desktop';
import { isDesktopMockActive } from './invoke/dispatch';

// DesktopApi外のスタンドアロンコマンド。Linuxのavailableはサービスへの接続だけで、
// OS設定による表示許可を保証しない。mockの既定は従来Windows経路を表すgranted。
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
