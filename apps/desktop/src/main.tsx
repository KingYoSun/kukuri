import React from 'react';
import ReactDOM from 'react-dom/client';

import { App } from './App';
import './styles.css';

if (import.meta.env.DEV) {
  console.info('[kukuri.desktop] frontend boot');
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
