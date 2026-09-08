import { useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { firstUnconsentedCommunityNode } from '@/lib/api/communityNodeAvailability';
import { useDesktopShellStore, useDesktopShellStoreApi } from '@/shell/store';

export function useCommunityNodeOnboarding() {
  const store = useDesktopShellStoreApi();
  const state = useDesktopShellStore(useShallow((s) => ({
    config: s.communityNodeConfig, statuses: s.communityNodeStatuses,
    loaded: s.communityNodeConfigLoaded && s.communityNodeStatusesLoaded &&
      !s.communityNodeConfigError && !s.communityNodeStatusError,
    author: s.syncStatus.local_author_pubkey,
    shownFor: s.communityNodeOnboardingShownFor,
    settingsOpen: s.shellChromeState.settingsOpen,
  })));
  const candidate = firstUnconsentedCommunityNode(state.config, state.statuses, state.loaded);
  const [open, setOpen] = useState(false);
  const returnFocus = useRef<HTMLElement | null>(null);
  const handingOff = useRef(false);
  const shown = state.shownFor.includes(state.author);

  useEffect(() => {
    if (!candidate || !state.author || shown || state.settingsOpen) return;
    const tryOpen = () => {
      const modalOpen = [...document.querySelectorAll('[role="dialog"], [role="alertdialog"]')]
        .some((element) => !element.closest('[aria-hidden="true"], [hidden], [data-state="closed"], [data-open="false"]'));
      if (modalOpen) return;
      if (store.getState().communityNodeOnboardingShownFor.includes(state.author)) return;
      returnFocus.current = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      handingOff.current = false;
      store.getState().setField('communityNodeOnboardingShownFor', (current) => [...current, state.author]);
      setOpen(true);
    };
    tryOpen();
    // 別のmodalが操作中なら終了まで待つ。pollや追加の外部I/Oはしない。
    const observer = new MutationObserver(tryOpen);
    observer.observe(document.body, { childList: true, subtree: true, attributes: true,
      attributeFilter: ['data-state', 'aria-hidden', 'hidden', 'data-open'] });
    return () => observer.disconnect();
  }, [candidate, shown, state.author, state.settingsOpen, store]);

  useEffect(() => { if (!candidate) setOpen(false); }, [candidate]);

  return {
    baseUrl: open ? candidate : null,
    dismiss: () => setOpen(false),
    handOff: () => {
      handingOff.current = true;
      setOpen(false);
      return returnFocus.current;
    },
    restoreFocus: (event: Event) => {
      event.preventDefault();
      if (!handingOff.current && returnFocus.current?.isConnected) returnFocus.current.focus();
    },
  };
}
