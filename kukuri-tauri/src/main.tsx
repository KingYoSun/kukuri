import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import { offlineSyncService } from './services/offlineSyncService';

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
