import { registerE2EBridge } from './registerE2EBridge';

declare global {
  interface Window {
    __KUKURI_E2E_BOOTSTRAP__?: () => Promise<void> | void;
  }
}

if (typeof window !== 'undefined' && !window.__KUKURI_E2E_BOOTSTRAP__) {
  window.__KUKURI_E2E_BOOTSTRAP__ = registerE2EBridge;
}
