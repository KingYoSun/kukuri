import { useCallback, useEffect, useRef, useState } from 'react';

import type { DesktopApi } from '@/lib/api';
import { lastDesktopLogSeq, mergeDesktopLogSnapshot, type DesktopLogsView } from '@/lib/desktopLogs';

export type DesktopLogsStatus = 'idle' | 'loading' | 'refreshing' | 'ready' | 'error';

export type DesktopLogsState = {
  status: DesktopLogsStatus;
  view: DesktopLogsView | null;
  errorMessage: string | null;
};

const IDLE_STATE: DesktopLogsState = { status: 'idle', view: null, errorMessage: null };

// #978: 開発者モード ON のあいだだけ、表示時と明示的な「更新」で ring buffer を読む。
// polling は行わない。OFF になったら表示を捨て、IPC も呼ばない(AC-1)。
export function useDesktopLogs(api: Pick<DesktopApi, 'readDesktopLogs'>, enabled: boolean) {
  const [state, setState] = useState<DesktopLogsState>(IDLE_STATE);
  const viewRef = useRef<DesktopLogsView | null>(null);
  // 無効化・unmount 後に届いた応答を捨てるための世代番号。
  const requestRef = useRef(0);

  const refresh = useCallback(async () => {
    if (!enabled) {
      return;
    }
    const request = ++requestRef.current;
    const previous = viewRef.current;
    setState((current) => ({
      status: previous ? 'refreshing' : 'loading',
      view: current.view,
      errorMessage: null,
    }));
    try {
      const snapshot = await api.readDesktopLogs(lastDesktopLogSeq(previous), null);
      if (request !== requestRef.current) {
        return;
      }
      const view = mergeDesktopLogSnapshot(previous, snapshot);
      viewRef.current = view;
      setState({ status: 'ready', view, errorMessage: null });
    } catch (error) {
      if (request !== requestRef.current) {
        return;
      }
      setState({
        status: 'error',
        view: viewRef.current,
        errorMessage: error instanceof Error ? error.message : String(error),
      });
    }
  }, [api, enabled]);

  useEffect(() => {
    if (!enabled) {
      requestRef.current += 1;
      viewRef.current = null;
      setState(IDLE_STATE);
      return;
    }
    void refresh();
    return () => {
      requestRef.current += 1;
    };
  }, [enabled, refresh]);

  return { ...state, refresh };
}
