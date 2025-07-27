import { createRootRoute, Outlet, useNavigate } from '@tanstack/react-router';
import { MainLayout } from '@/components/layout/MainLayout';
import { useTopics, useP2P, useAuth } from '@/hooks';
import { useEffect, useState } from 'react';
import { useAuthStore } from '@/stores/authStore';

function RootComponent() {
  const navigate = useNavigate();
  const { isAuthenticated, initialize } = useAuthStore();
  const { data: topics, isLoading } = useTopics();
  const { initialized: p2pInitialized } = useP2P();
  const [isInitializing, setIsInitializing] = useState(true);

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
      const pathname = window.location.pathname;
      
      // 認証が必要なページのリスト
      const authRequiredPaths = ['/', '/topics', '/settings'];
      const authPaths = ['/welcome', '/login', '/profile-setup'];
      
      if (!isAuthenticated && authRequiredPaths.some(path => pathname.startsWith(path))) {
        // 未認証でprotectedページにアクセスしようとした場合
        navigate({ to: '/welcome' });
      } else if (isAuthenticated && authPaths.includes(pathname)) {
        // 認証済みで認証ページにアクセスしようとした場合
        navigate({ to: '/' });
      }
    }
  }, [isAuthenticated, isInitializing, navigate]);

  useEffect(() => {
    // 初期トピックデータの読み込み
    if (topics) {
      console.log('Topics loaded:', topics);
    }
  }, [topics]);

  useEffect(() => {
    // P2P初期化の確認
    console.log('P2P initialized:', p2pInitialized);
  }, [p2pInitialized]);

  if (isInitializing) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-muted-foreground">初期化中...</p>
      </div>
    );
  }

  // 認証が必要なページで未認証の場合
  const pathname = window.location.pathname;
  const isProtectedRoute = ['/', '/topics', '/settings'].some(path => pathname.startsWith(path));
  
  if (!isAuthenticated && isProtectedRoute) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-muted-foreground">リダイレクト中...</p>
      </div>
    );
  }

  // 認証ページの場合はレイアウトなしで表示
  const isAuthPage = ['/welcome', '/login', '/profile-setup'].includes(pathname);
  
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
