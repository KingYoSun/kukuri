import { useMemo, useState } from 'react';

import { DesktopShellPage } from '@/shell/DesktopShellPage';
import {
  createDesktopShellStore,
  DesktopShellStoreContext,
} from '@/shell/store';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import { DEVELOPER_MODE_STORAGE_KEY } from '@/lib/developerMode';
import type { DesktopTheme } from '@/lib/theme';

function createReviewStore() {
  window.localStorage.setItem(DEVELOPER_MODE_STORAGE_KEY, 'true');
  return createDesktopShellStore();
}

export function ProductionColumnWorkspaceStory() {
  const [store] = useState(createReviewStore);
  const api = useMemo(() => createDesktopMockApi(), []);
  const [theme, setTheme] = useState<DesktopTheme>('dark');

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <DesktopShellPage api={api} theme={theme} onThemeChange={setTheme} />
    </DesktopShellStoreContext.Provider>
  );
}
