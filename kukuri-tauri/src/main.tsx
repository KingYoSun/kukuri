import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { offlineSyncService } from './services/offlineSyncService';
import './testing/e2eBootstrap';

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
