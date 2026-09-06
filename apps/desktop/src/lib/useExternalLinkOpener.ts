import { useRef, useState, type MouseEvent } from 'react';

import { invokeDesktop } from '@/lib/api/invoke/desktop';
import { isTauriRuntime } from '@/lib/releaseReadiness';

// Only attach to external resource links, never internal navigation/downloads.
export function useExternalLinkOpener() {
  const inFlight = useRef(false);
  const [pending, setPending] = useState(false);
  const [failed, setFailed] = useState(false);

  const onClick = (event: MouseEvent<HTMLAnchorElement>) => {
    if (event.defaultPrevented || (event.button !== 0 && event.button !== 1)) return;
    if (!isTauriRuntime()) return;
    event.preventDefault();
    if (inFlight.current) return;
    // Do not silently resolve malformed input against the app's origin.
    const url = event.currentTarget.getAttribute('href');
    if (!url) return;
    inFlight.current = true;
    setPending(true);
    setFailed(false);
    void invokeDesktop<void>('open_external_url', { url })
      .catch(() => setFailed(true))
      .finally(() => {
        inFlight.current = false;
        setPending(false);
      });
  };

  return {
    pending,
    failed,
    linkProps: { onClick, onAuxClick: onClick, 'aria-disabled': pending || undefined },
  };
}
