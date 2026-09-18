import React from 'react';
import ReactDOM from 'react-dom/client';

import { initializeDesktopLocale } from '@/i18n/bootstrap';
import { browserDesktopMockSeed } from '@/mocks/browserSeed';
import { type DesktopMockApiOptions } from '@/mocks/desktopApiMock';
import { installWindowDesktopMock } from '@/mocks/installWindowDesktopMock';
import { App } from './App';
import '@/styles/index.css';

if (import.meta.env.VITE_KUKURI_DESKTOP_MOCK === '1') {
  // 告知素材の撮影 (#1039) は、撮影前に window へ置いた seed で mock を作る。
  // 読むのは mock 経路の中だけなので、本番 bundle には入らない。
  const promoSeed = (window as { __KUKURI_PROMO_MOCK_SEED__?: DesktopMockApiOptions })
    .__KUKURI_PROMO_MOCK_SEED__;
  installWindowDesktopMock(promoSeed ?? browserDesktopMockSeed);
}

if (import.meta.env.DEV) {
  console.info('[kukuri.desktop] frontend boot');
}

void initializeDesktopLocale().then(() => {
  ReactDOM.createRoot(document.getElementById('root')!).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>
  );
});
