import './App.css';
import { RouterProvider } from '@tanstack/react-router';
import { QueryClientProvider } from '@tanstack/react-query';
import { router } from './router';
import { queryClient } from './lib/queryClient';
import { Toaster } from 'sonner';
import { OfflineIndicator } from './components/OfflineIndicator';
import { AppErrorBoundary } from './components/AppErrorBoundary';
import { usePrivacySettingsAutoSync } from './hooks/usePrivacySettingsAutoSync';

function App() {
  usePrivacySettingsAutoSync();

  return (
    <QueryClientProvider client={queryClient}>
      <AppErrorBoundary>
        <RouterProvider router={router} />
      </AppErrorBoundary>
      <OfflineIndicator />
      <Toaster />
    </QueryClientProvider>
  );
}

export default App;
