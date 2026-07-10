import { useEffect, useRef } from 'react';

type UseFocusScrollArgs = {
  focusKey: string | null;
  readinessKey: unknown;
  selector: string | null;
};

export function useFocusScroll({
  focusKey,
  readinessKey,
  selector,
}: UseFocusScrollArgs): void {
  const lastFocusedKeyRef = useRef<string | null>(null);

  useEffect(() => {
    if (!focusKey || !selector || lastFocusedKeyRef.current === focusKey) {
      return;
    }
    const frameId = window.requestAnimationFrame(() => {
      const target = document.querySelector(selector);
      if (!(target instanceof HTMLElement)) {
        return;
      }
      if (typeof target.scrollIntoView === 'function') {
        target.scrollIntoView({ block: 'center' });
      }
      target.focus({ preventScroll: true });
      lastFocusedKeyRef.current = focusKey;
    });
    return () => window.cancelAnimationFrame(frameId);
  }, [focusKey, readinessKey, selector]);
}
