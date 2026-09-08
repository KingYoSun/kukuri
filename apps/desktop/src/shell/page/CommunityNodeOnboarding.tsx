import { useShallow } from 'zustand/react/shallow';
import type { DesktopApi } from '@/lib/api';
import { communityIndexNodeLabel } from '@/lib/api/communityIndex';
import { communityIndexAvailability, type CommunityNodeAvailability } from '@/lib/api/communityNodeAvailability';
import { CommunityNodeConsentDialog } from '@/components/settings/CommunityNodeConsentDialog';
import { CommunityNodeOnboardingDialog } from '@/components/settings/CommunityNodeOnboardingDialog';
import { CommunityIndexAvailabilityNotice } from '@/components/core/CommunityIndexAvailabilityNotice';
import { useCommunityNodeConsentFlow, type AcceptCommunityNodeConsents } from '@/shell/actions/useCommunityNodeConsentFlow';
import { useDesktopShellFieldSetter, useDesktopShellStore } from '@/shell/store';
import { useCommunityNodeOnboarding } from './useCommunityNodeOnboarding';

export function CommunityNodeOnboarding({ api, onAccept, onOpenSettings, onRetry }: {
  api: DesktopApi;
  onAccept: AcceptCommunityNodeConsents;
  onOpenSettings: () => void;
  onRetry: (availability: CommunityNodeAvailability) => Promise<void>;
}) {
  const intro = useCommunityNodeOnboarding();
  const setPreference = useDesktopShellFieldSetter('communityIndexNodePreference');
  const state = useDesktopShellStore(useShallow((s) => ({
    config: s.communityNodeConfig, statuses: s.communityNodeStatuses, manifests: s.communityNodeManifests,
    preference: s.communityIndexNodePreference, configLoaded: s.communityNodeConfigLoaded,
    statusesLoaded: s.communityNodeStatusesLoaded,
    statusError: Boolean(s.communityNodeConfigError || s.communityNodeStatusError),
  })));
  const consent = useCommunityNodeConsentFlow({
    api, configuredBaseUrls: state.config.nodes.map((node) => node.base_url),
    statuses: state.statuses, acceptConsents: onAccept,
  });
  const baseUrl = intro.baseUrl;
  const availability = communityIndexAvailability(state);
  return <>
    {state.statusError ? <CommunityIndexAvailabilityNotice
      availability={availability} onRetry={() => onRetry(availability)} onReviewPolicies={consent.open}
      onOpenSettings={onOpenSettings} onAutomatic={() => setPreference({ mode: 'auto' })}
    /> : null}
    {baseUrl ? <CommunityNodeOnboardingDialog
      baseUrl={baseUrl}
      nodeLabel={state.manifests[baseUrl]?.status === 'ok'
        ? communityIndexNodeLabel(baseUrl, state.manifests[baseUrl]) : new URL(baseUrl).host}
      onDismiss={intro.dismiss}
      onReview={() => consent.open(baseUrl, intro.handOff())}
      onOpenSettings={() => { intro.handOff(); onOpenSettings(); }}
      onCloseAutoFocus={intro.restoreFocus}
    /> : null}
    {consent.dialog ? <CommunityNodeConsentDialog {...consent.dialog} /> : null}
  </>;
}
