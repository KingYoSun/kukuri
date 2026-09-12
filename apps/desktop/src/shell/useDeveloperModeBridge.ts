import { useEffect } from 'react';

import { invokeDesktop } from '@/lib/api/invoke/desktop';
import { isTauriRuntime } from '@/lib/releaseReadiness';

// #978: 開発者モードの正本は frontend の localStorage のまま。backend はログ閲覧 IPC を
// 開発者モード OFF で拒否するため、現在値を mount 時と変更時に in-memory ミラーへ写す。
// backend は永続化しないので、再起動・アカウント切替後の再 mount でも必ず送り直す。
export function useDeveloperModeBridge(enabled: boolean): void {
  useEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    void invokeDesktop<void>('set_developer_mode_enabled', { enabled }).catch(() => {
      // best effort mirror: Ready 以外では gate が拒否し、次の変更で再送する。
    });
  }, [enabled]);
}
