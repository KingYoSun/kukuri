import { ReactNode } from 'react';
import { Header } from '@/components/layout/Header';
import { Sidebar } from '@/components/layout/Sidebar';
import { useUIStore } from '@/stores';
import { cn } from '@/lib/utils';
import { GlobalComposer } from '@/components/posts/GlobalComposer';
import { DirectMessageDialog } from '@/components/directMessages/DirectMessageDialog';
import { useTheme } from '@/hooks/useTheme';

interface MainLayoutProps {
  children: ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { sidebarOpen } = useUIStore();
  useTheme(); // Apply theme to HTML element

  return (
    <div className="h-screen flex flex-col bg-background">
      <Header />
      <div className="flex flex-1 overflow-hidden">
        <div
          className={cn('transition-all duration-300', sidebarOpen ? 'w-64' : 'w-0')}
          data-testid="sidebar"
        >
          <Sidebar />
        </div>
        <main className="flex-1 overflow-auto">
          <div className="container mx-auto p-6">{children}</div>
        </main>
      </div>
      <GlobalComposer />
      <DirectMessageDialog />
    </div>
  );
}
