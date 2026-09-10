import { useEffect, useRef, useState } from 'react';
import { useShallow } from 'zustand/react/shallow';
import { TesterFeedbackDialog } from '@/components/core/TesterFeedbackDialog';
import type { DesktopApi } from '@/lib/api';
import { eligibleTesterFeedbackNodes } from '@/lib/api/communityIndex';
import { testerFeedbackAvailability } from '@/lib/api/testerFeedbackAvailability';
import { SHELL_SETTINGS_ID, useDesktopShellStore } from '@/shell/store';

export function DesktopShellTesterFeedbackDialog({ api, open, onOpenChange, onOpenCommunityNodeSettings }: {
  api: DesktopApi;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenCommunityNodeSettings: () => void;
}) {
  const state = useDesktopShellStore(useShallow(store => ({
    config: store.communityNodeConfig,
    statuses: store.communityNodeStatuses,
    manifests: store.communityNodeManifests,
    configLoaded: store.communityNodeConfigLoaded,
    configError: Boolean(store.communityNodeConfigError),
    statusesLoaded: store.communityNodeStatusesLoaded,
    statusError: Boolean(store.communityNodeStatusError),
    settingsOpen: store.shellChromeState.settingsOpen,
  })));
  const settingsFocusRequested = useRef(false);
  const [focusRequest, setFocusRequest] = useState(0);
  useEffect(() => {
    if (!state.settingsOpen || !settingsFocusRequested.current) return;
    settingsFocusRequested.current = false;
    const drawer = document.getElementById(SHELL_SETTINGS_ID);
    let cancelled = false;
    // visibilityのtransition中はfocusが拒否されるため、描画完了を待つ。
    void Promise.allSettled((drawer?.getAnimations?.() ?? []).map(animation => animation.finished)).then(() => {
      if (!cancelled && drawer?.dataset.open === 'true') {
        drawer.querySelector<HTMLElement>('.shell-settings-close')?.focus();
      }
    });
    return () => { cancelled = true; };
  }, [state.settingsOpen, focusRequest]);

  return <TesterFeedbackDialog
    api={api}
    open={open}
    onOpenChange={onOpenChange}
    eligibleNodeBaseUrls={eligibleTesterFeedbackNodes(state.config, state.statuses, state.manifests)}
    availability={testerFeedbackAvailability(state)}
    onOpenCommunityNodeSettings={() => {
      settingsFocusRequested.current = true;
      setFocusRequest(current => current + 1);
      onOpenCommunityNodeSettings();
    }}
  />;
}
