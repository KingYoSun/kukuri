import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { RefreshCw } from 'lucide-react';

import { IconButton } from '@/components/ui/icon-button';

type ProfileRefreshButtonProps = {
  refreshing: boolean;
  saving: boolean;
  onRefresh: () => Promise<void>;
};

export function ProfileRefreshButton({ refreshing, saving, onRefresh }: ProfileRefreshButtonProps) {
  const { t } = useTranslation('profile');
  const [feedbackPending, setFeedbackPending] = useState(false);
  const timeout = useRef<ReturnType<typeof setTimeout> | null>(null);
  const showFeedback = useCallback(() => {
    if (timeout.current !== null) clearTimeout(timeout.current);
    setFeedbackPending(true);
    timeout.current = setTimeout(() => {
      timeout.current = null;
      setFeedbackPending(false);
    }, 1000);
  }, []);

  useEffect(() => {
    if (refreshing) showFeedback();
  }, [refreshing, showFeedback]);
  useEffect(() => () => {
    if (timeout.current !== null) clearTimeout(timeout.current);
  }, []);

  // Keep acknowledgement visible without delaying data application in the loader.
  const busy = refreshing || feedbackPending;
  return (
    <IconButton
      variant='ghost'
      type='button'
      label={t(busy ? 'overview.refreshing' : 'overview.refresh')}
      aria-busy={busy}
      aria-disabled={busy || saving}
      onClick={() => {
        if (busy || saving) return;
        showFeedback();
        void onRefresh();
      }}
    >
      <RefreshCw className={`size-4${busy ? ' profile-refresh-spinning' : ''}`} aria-hidden='true' />
    </IconButton>
  );
}
