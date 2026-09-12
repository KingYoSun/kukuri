import { useCallback, useEffect, useRef, useState } from 'react';

/** DESIGN.md 4節: 開始から最低この時間はpendingを示す。結果の反映は遅らせない。 */
export const ACKNOWLEDGED_PENDING_MS = 1000;

/**
 * 実処理が短時間で終わっても、開始の受付を最低1秒間pendingとして示す。
 * `active` が true の間は実処理が続いているので、その終了まで busy を維持する。
 */
export function useAcknowledgedPending(active: boolean): { busy: boolean; acknowledge: () => void } {
  const [feedbackPending, setFeedbackPending] = useState(false);
  const timeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const acknowledge = useCallback(() => {
    if (timeout.current !== null) clearTimeout(timeout.current);
    setFeedbackPending(true);
    timeout.current = setTimeout(() => {
      timeout.current = null;
      setFeedbackPending(false);
    }, ACKNOWLEDGED_PENDING_MS);
  }, []);

  useEffect(() => {
    if (active) acknowledge();
  }, [active, acknowledge]);
  useEffect(() => () => {
    if (timeout.current !== null) clearTimeout(timeout.current);
  }, []);

  // Keep acknowledgement visible without delaying data application in the caller.
  return { busy: active || feedbackPending, acknowledge };
}
