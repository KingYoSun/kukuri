import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { errorHandler } from './lib/errorHandler';
import { offlineSyncService } from './services/offlineSyncService';
import { registerE2EBridge } from './testing/registerE2EBridge';
import { setE2EStatus, type E2EStatus } from './testing/e2eStatus';

declare global {
  interface Window {
    __KUKURI_E2E_STATUS__?: E2EStatus;
  }
}

const enableE2EBridge =
  import.meta.env.TAURI_ENV_DEBUG === 'true' || import.meta.env.VITE_ENABLE_E2E === 'true';

if (enableE2EBridge) {
  if (typeof window !== 'undefined' && !window.__KUKURI_E2E_BOOTSTRAP__) {
    window.__KUKURI_E2E_BOOTSTRAP__ = registerE2EBridge;
  }

  setE2EStatus('pending');

  try {
    registerE2EBridge();

    setE2EStatus('registered');
  } catch (error) {
    errorHandler.log('[E2E] Failed to register E2E bridge', error, { context: 'E2EBridge' });
    setE2EStatus('error');
  }
} else {
  setE2EStatus('disabled');
}

// Initialize offline sync service
offlineSyncService.initialize();

// Register cleanup handler
window.addEventListener('beforeunload', () => {
  offlineSyncService.cleanup();
});

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
