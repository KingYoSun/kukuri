import { createRootRoute, Outlet, useNavigate, useLocation } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { MainLayout } from '@/components/layout/MainLayout';
import { AppErrorBoundary } from '@/components/AppErrorBoundary';
import { useTopics, useP2P, useProfileAvatarSync } from '@/hooks';
import { useNostrEvents } from '@/hooks/useNostrEvents';
import { useDataSync } from '@/hooks/useDataSync';
import { useDirectMessageEvents } from '@/hooks/useDirectMessageEvents';
import { useEffect, useState } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { errorHandler } from '@/lib/errorHandler';

const PROTECTED_PATHS = ['/topics', '/settings', '/profile-setup'];
const AUTH_REDIRECT_PATHS = ['/welcome'];
const AUTH_LAYOUT_PATHS = ['/welcome', '/login', '/profile-setup'];
const clampText = (value: string, max = 500) => (value.length > max ? value.slice(0, max) : value);

function RootLoadingMessage() {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-center h-screen">
      <p className="text-muted-foreground">{t('root.initializing')}</p>
    </div>
  );
}

function RootRedirectingMessage() {
  const { t } = useTranslation();
  return (
    <div className="flex items-center justify-center h-screen">
      <p className="text-muted-foreground">{t('root.redirecting')}</p>
    </div>
  );
}

function RootErrorComponent({ error }: { error: unknown }) {
  useEffect(() => {
    errorHandler.log('RootRoute render failed', error, {
      context: 'RootRoute.error',
    });
    if (typeof document !== 'undefined' && document.documentElement) {
      const message =
        error instanceof Error ? (error.stack ?? error.message ?? 'Unknown error') : String(error);
      document.documentElement.setAttribute('data-kukuri-e2e-error', clampText(message));
    }
  }, [error]);

  const message = error instanceof Error ? error.message : String(error);

  const { t } = useTranslation();
  return (
    <div className="p-6" data-testid="root-error-boundary">
      <h1 className="text-lg font-semibold">{t('root.errorTitle')}</h1>
      {message ? <p className="mt-2 text-sm text-muted-foreground">{message}</p> : null}
    </div>
  );
}

function RootComponent() {
  const navigate = useNavigate();
  const location = useLocation();
  const { isAuthenticated, initialize } = useAuthStore();
  const [isInitializing, setIsInitializing] = useState(true);

  useEffect(() => {
    const initApp = async () => {
      await initialize();
      setIsInitializing(false);
    };
    initApp();
  }, [initialize]);

  useEffect(() => {
    if (!isInitializing) {
      const pathname = location.pathname;

      const isRootPath = pathname === '/';
      const isAuthRequiredPath =
        isRootPath || PROTECTED_PATHS.some((path) => pathname.startsWith(path));

      if (!isAuthenticated && isAuthRequiredPath) {
        errorHandler.info(
          `Redirecting unauthenticated user from ${pathname} to /welcome`,
          'RootRoute.authGuard',
        );
        navigate({ to: '/welcome' });
      } else if (isAuthenticated && AUTH_REDIRECT_PATHS.includes(pathname)) {
        errorHandler.info(
          `Redirecting authenticated user from ${pathname} to /`,
          'RootRoute.authGuard',
        );
        navigate({ to: '/' });
      }
    }
  }, [isAuthenticated, isInitializing, navigate, location.pathname]);

  if (isInitializing) {
    return <RootLoadingMessage />;
  }

  const pathname = location.pathname;
  const isRootPath = pathname === '/';
  const isProtectedRoute = isRootPath || PROTECTED_PATHS.some((path) => pathname.startsWith(path));

  if (!isAuthenticated && isProtectedRoute) {
    return <RootRedirectingMessage />;
  }

  const isAuthPage = AUTH_LAYOUT_PATHS.includes(pathname);

  if (isAuthPage) {
    return (
      <AppErrorBoundary>
        <Outlet />
      </AppErrorBoundary>
    );
  }

  return <MainAppShell />;
}

function MainAppShell() {
  const { t } = useTranslation();
  const { data: topics, isLoading } = useTopics();
  const { initialized: p2pInitialized } = useP2P();

  useProfileAvatarSync();
  useNostrEvents();
  useDirectMessageEvents();
  useDataSync();

  useEffect(() => {
    if (topics) {
      errorHandler.info('Topics loaded', 'RootRoute.topicsEffect');
    }
  }, [topics]);

  useEffect(() => {
    errorHandler.info(`P2P initialized: ${p2pInitialized}`, 'RootRoute.p2pEffect');
  }, [p2pInitialized]);
  if (isLoading) {
    return (
      <MainLayout>
        <div className="flex items-center justify-center h-screen">
          <p className="text-muted-foreground">{t('common.loading')}</p>
        </div>
      </MainLayout>
    );
  }

  return (
    <AppErrorBoundary>
      <MainLayout>
        <Outlet />
      </MainLayout>
    </AppErrorBoundary>
  );
}

export const Route = createRootRoute({
  component: RootComponent,
  errorComponent: RootErrorComponent,
});
