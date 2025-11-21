import { createRootRoute, Outlet, useNavigate, useLocation } from '@tanstack/react-router';
import { MainLayout } from '@/components/layout/MainLayout';
import { useTopics, useP2P, useProfileAvatarSync } from '@/hooks';
import { useNostrEvents } from '@/hooks/useNostrEvents';
import { useP2PEventListener } from '@/hooks/useP2PEventListener';
import { useDataSync } from '@/hooks/useDataSync';
import { useDirectMessageEvents } from '@/hooks/useDirectMessageEvents';
import { useEffect, useState } from 'react';
import { useAuthStore } from '@/stores/authStore';
import { errorHandler } from '@/lib/errorHandler';
import { registerProfileAvatarSyncWorker } from '@/serviceWorker/profileAvatarSyncBridge';

const PROTECTED_PATHS = ['/topics', '/settings', '/profile-setup'];
const AUTH_REDIRECT_PATHS = ['/welcome', '/login'];
const AUTH_LAYOUT_PATHS = [...AUTH_REDIRECT_PATHS, '/profile-setup'];

function RootComponent() {
  const navigate = useNavigate();
  const location = useLocation();
  const { isAuthenticated, initialize } = useAuthStore();
  const { data: topics, isLoading } = useTopics();
  const { initialized: p2pInitialized } = useP2P();
  const [isInitializing, setIsInitializing] = useState(true);
  useProfileAvatarSync();

  // グローバルイベントリスナーの設定
  useNostrEvents();
  useP2PEventListener();
  useDirectMessageEvents();

  // データ同期の設定
  useDataSync();

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }
    void registerProfileAvatarSyncWorker();
  }, []);

  useEffect(() => {
    // アプリ起動時の初期化
    const initApp = async () => {
      await initialize();
      setIsInitializing(false);
    };
    initApp();
  }, [initialize]);

  useEffect(() => {
    // 初期化完了後、認証状態によるリダイレクト
    if (!isInitializing) {
      const pathname = location.pathname;

      // ルートパスの特別な処理
      const isRootPath = pathname === '/';
      const isAuthRequiredPath =
        isRootPath || PROTECTED_PATHS.some((path) => pathname.startsWith(path));

      if (!isAuthenticated && isAuthRequiredPath) {
        // 未認証でprotectedページにアクセスしようとした場合
        errorHandler.info(
          `Redirecting unauthenticated user from ${pathname} to /welcome`,
          'RootRoute.authGuard',
        );
        navigate({ to: '/welcome' });
      } else if (isAuthenticated && AUTH_REDIRECT_PATHS.includes(pathname)) {
        // 認証済みで認証ページ（welcome/login）にアクセスしようとした場合
        errorHandler.info(
          `Redirecting authenticated user from ${pathname} to /`,
          'RootRoute.authGuard',
        );
        navigate({ to: '/' });
      }
    }
  }, [isAuthenticated, isInitializing, navigate, location.pathname]);

  useEffect(() => {
    // 初期トピックデータの読み込み
    if (topics) {
      errorHandler.info('Topics loaded', 'RootRoute.topicsEffect');
    }
  }, [topics]);

  useEffect(() => {
    // P2P初期化の確認
    errorHandler.info(`P2P initialized: ${p2pInitialized}`, 'RootRoute.p2pEffect');
  }, [p2pInitialized]);

  if (isInitializing) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-muted-foreground">初期化中...</p>
      </div>
    );
  }

  // 認証が必要なページで未認証の場合
  const pathname = location.pathname;
  const isRootPath = pathname === '/';
  const isProtectedRoute = isRootPath || PROTECTED_PATHS.some((path) => pathname.startsWith(path));

  if (!isAuthenticated && isProtectedRoute) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-muted-foreground">リダイレクト中...</p>
      </div>
    );
  }

  // 認証ページの場合はレイアウトなしで表示
  const isAuthPage = AUTH_LAYOUT_PATHS.includes(pathname);

  if (isAuthPage) {
    return <Outlet />;
  }

  // 通常ページはレイアウトありで表示
  if (isLoading) {
    return (
      <MainLayout>
        <div className="flex items-center justify-center h-screen">
          <p className="text-muted-foreground">読み込み中...</p>
        </div>
      </MainLayout>
    );
  }

  return (
    <MainLayout>
      <Outlet />
    </MainLayout>
  );
}

export const Route = createRootRoute({
  component: RootComponent,
});
