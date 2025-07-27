import { createRootRoute, Outlet } from '@tanstack/react-router';
import { MainLayout } from '@/components/layout/MainLayout';
import { useTopics, useP2P } from '@/hooks';
import { useEffect } from 'react';

function RootComponent() {
  const { data: topics, isLoading } = useTopics();
  const { initialized: p2pInitialized } = useP2P();

  useEffect(() => {
    // 初期トピックデータの読み込み
    console.log('Topics loaded:', topics);
  }, [topics]);

  useEffect(() => {
    // P2P初期化の確認
    console.log('P2P initialized:', p2pInitialized);
  }, [p2pInitialized]);

  if (isLoading) {
    return (
      <div className="flex items-center justify-center h-screen">
        <p className="text-muted-foreground">読み込み中...</p>
      </div>
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
