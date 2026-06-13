import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { HashRouter } from 'react-router-dom';

import { Button } from '@/components/ui/button';
import { Notice } from '@/components/ui/notice';
import { DesktopShellPage } from '@/shell/DesktopShellPage';
import {
  type AppProps,
  DesktopShellStoreContext,
  createDesktopShellStore,
} from '@/shell/store';
import {
  type DesktopStartupErrorView,
  type DesktopStartupStatus,
  getDesktopStartupStatus,
} from '@/lib/api';
import { BACKEND_UNAVAILABLE_MESSAGE } from '@/lib/api/invoke/error';
import {
  type DesktopTheme,
  readDesktopTheme,
  writeDesktopTheme,
} from '@/lib/theme';
import { copyTextToClipboard } from '@/lib/utils';

type StartupGateState =
  | { status: 'checking' }
  | { status: 'ready' }
  | { status: 'failed'; error: DesktopStartupErrorView };

export function App(props: AppProps) {
  const [store] = useState(() => createDesktopShellStore());
  const [theme, setTheme] = useState<DesktopTheme>(() => readDesktopTheme());
  const [startupGate, setStartupGate] = useState<StartupGateState>(() =>
    props.api ? { status: 'ready' } : { status: 'checking' }
  );

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
  }, [theme]);

  useEffect(() => {
    writeDesktopTheme(theme);
  }, [theme]);

  useEffect(() => {
    if (props.api) {
      setStartupGate({ status: 'ready' });
      return;
    }

    let active = true;
    getDesktopStartupStatus()
      .then((status: DesktopStartupStatus) => {
        if (!active) {
          return;
        }
        setStartupGate(status.status === 'failed' ? status : { status: 'ready' });
      })
      .catch((error: unknown) => {
        if (!active) {
          return;
        }
        if (error instanceof Error && error.message === BACKEND_UNAVAILABLE_MESSAGE) {
          setStartupGate({ status: 'ready' });
          return;
        }
        setStartupGate({
          status: 'failed',
          error: {
            kind: 'unknown',
            message: 'kukuri could not finish desktop startup.',
            detail: error instanceof Error ? error.message : String(error),
            db_path: null,
          },
        });
      });

    return () => {
      active = false;
    };
  }, [props.api]);

  if (startupGate.status === 'checking') {
    return <StartupStatusScreen status='checking' />;
  }

  if (startupGate.status === 'failed') {
    return <StartupStatusScreen status='failed' error={startupGate.error} />;
  }

  return (
    <DesktopShellStoreContext.Provider value={store}>
      <HashRouter>
        <DesktopShellPage {...props} theme={theme} onThemeChange={setTheme} />
      </HashRouter>
    </DesktopShellStoreContext.Provider>
  );
}

function StartupStatusScreen({
  status,
  error,
}: {
  status: 'checking' | 'failed';
  error?: DesktopStartupErrorView;
}) {
  const { t } = useTranslation(['common']);
  const detail = error
    ? [
        `kind: ${error.kind}`,
        `db_path: ${error.db_path ?? 'unknown'}`,
        '',
        error.detail,
      ].join('\n')
    : '';

  return (
    <main className='startup-error-screen'>
      <section className='startup-error-panel' aria-live='polite'>
        {status === 'checking' ? (
          <Notice>{t('startup.checking')}</Notice>
        ) : (
          <>
            <Notice tone='destructive'>
              <strong>{t('startup.title')}</strong>
              <span>{t('startup.description')}</span>
            </Notice>
            <div className='startup-error-actions'>
              <Button type='button' onClick={() => window.location.reload()}>
                {t('actions.retry')}
              </Button>
              <Button
                type='button'
                variant='secondary'
                onClick={() => void copyTextToClipboard(detail)}
              >
                {t('startup.copyDetails')}
              </Button>
            </div>
            <dl className='startup-error-summary'>
              <div>
                <dt>{t('startup.kind')}</dt>
                <dd>{t(`startup.kinds.${error?.kind ?? 'unknown'}`)}</dd>
              </div>
              <div>
                <dt>{t('startup.dbPath')}</dt>
                <dd>{error?.db_path ?? t('fallbacks.unknown')}</dd>
              </div>
            </dl>
            <textarea
              className='startup-error-detail'
              value={detail}
              readOnly
              aria-label={t('startup.detailLabel')}
            />
          </>
        )}
      </section>
    </main>
  );
}
