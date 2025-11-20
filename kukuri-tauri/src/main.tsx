import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { offlineSyncService } from './services/offlineSyncService';

if (import.meta.env.TAURI_ENV_DEBUG === 'true') {
  import('./testing/registerE2EBridge').then(({ registerE2EBridge }) => registerE2EBridge());
}

// オフライン同期サービスの初期化
offlineSyncService.initialize();

// クリーンアップの設定
window.addEventListener('beforeunload', () => {
  offlineSyncService.cleanup();
});

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
