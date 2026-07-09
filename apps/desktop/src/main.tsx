import React from 'react';
import ReactDOM from 'react-dom/client';

import '@/i18n';
import { browserDesktopMockSeed } from '@/mocks/browserSeed';
import { installWindowDesktopMock } from '@/mocks/installWindowDesktopMock';
import { App } from './App';
import '@/styles/index.css';

if (import.meta.env.VITE_KUKURI_DESKTOP_MOCK === '1') {
  installWindowDesktopMock(browserDesktopMockSeed);
}

if (import.meta.env.DEV) {
  console.info('[kukuri.desktop] frontend boot');
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
