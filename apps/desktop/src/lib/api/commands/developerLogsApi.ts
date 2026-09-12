import type { DesktopApi, DesktopLogSnapshot } from '../types';
import { invokeDesktop } from '../invoke/desktop';
import { command } from '../invoke/dispatch';

// #978: 開発者モード ON 時だけ backend が応答する。OFF 時は呼ばない(backend も拒否する)。
// runtimeApi.ts の大型ファイル ratchet を増やさないため別 module に置く。
export const developerLogsApi: Pick<DesktopApi, 'readDesktopLogs'> = {
  readDesktopLogs: command('readDesktopLogs', (afterSeq, limit) =>
    invokeDesktop<DesktopLogSnapshot>('read_desktop_logs', {
      afterSeq: afterSeq ?? null,
      limit: limit ?? null,
    })),
};
