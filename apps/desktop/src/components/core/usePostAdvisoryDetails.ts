import { useCallback, useRef, useState } from 'react';

/// #1108: advisory 付き投稿の詳細 dialog の開閉と focus 復元。
///
/// 開く操作(メディア枠 / 本文欄の代替表示)は DialogTrigger ではないため、開いた要素を覚えて
/// 閉じたときに戻す。異議申し立てへ移った場合は詳細 dialog を閉じても focus を動かさず、
/// 通報 dialog を閉じた後に同じ要素へ戻す。
export function usePostAdvisoryDetails() {
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLElement | null>(null);
  const reportReturnFocusRef = useRef<HTMLElement | null>(null);

  const openDetails = useCallback((trigger: HTMLElement) => {
    triggerRef.current = trigger;
    setOpen(true);
  }, []);

  /// 詳細 dialog を閉じ、通報 dialog を閉じた後の focus 先として開いた要素を渡す。
  const handOffToReport = useCallback(() => {
    reportReturnFocusRef.current = triggerRef.current;
    setOpen(false);
  }, []);

  const skipReturnFocus = useCallback(() => reportReturnFocusRef.current !== null, []);

  const onReportCloseAutoFocus = useCallback((event: Event) => {
    const target = reportReturnFocusRef.current;
    if (!target) return;
    reportReturnFocusRef.current = null;
    event.preventDefault();
    target.focus();
  }, []);

  return {
    open,
    setOpen,
    triggerRef,
    openDetails,
    handOffToReport,
    skipReturnFocus,
    onReportCloseAutoFocus,
  };
}
