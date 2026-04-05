import { useEffect, useState } from 'react';
import { HashRouter } from 'react-router-dom';

import { DesktopShellPage } from '@/shell/DesktopShellPage';
import {
  type AppProps,
  DesktopShellStoreContext,
  createDesktopShellStore,
} from '@/shell/store';
import {
  type DesktopTheme,
  readDesktopTheme,
  writeDesktopTheme,
} from '@/lib/theme';

export function App(props: AppProps) {
  const [store] = useState(() => createDesktopShellStore());
  const [theme, setTheme] = useState<DesktopTheme>(() => readDesktopTheme());

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    writeDesktopTheme(theme);
  }, [theme]);

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <HashRouter>
        <DesktopShellPage {...props} theme={theme} onThemeChange={setTheme} />
      </HashRouter>
    </DesktopShellStoreContext.Provider>
  );
}
