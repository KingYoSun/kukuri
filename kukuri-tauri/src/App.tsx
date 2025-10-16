import './App.css';
import { RouterProvider } from '@tanstack/react-router';
import { QueryClientProvider } from '@tanstack/react-query';
import { router } from './router';
import { queryClient } from './lib/queryClient';
import { Toaster } from 'sonner';
import { OfflineIndicator } from './components/OfflineIndicator';

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
      <OfflineIndicator />
      <Toaster />
    </QueryClientProvider>
  );
}

export default App;
