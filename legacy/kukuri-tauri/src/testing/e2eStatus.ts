export type E2EStatus = 'pending' | 'registered' | 'error' | 'disabled';

export const setE2EStatus = (status: E2EStatus): void => {
  if (typeof window === 'undefined') {
    return;
  }

  window.__KUKURI_E2E_STATUS__ = status;
  if (typeof document !== 'undefined' && document.documentElement) {
    document.documentElement.setAttribute('data-kukuri-e2e-status', status);
  }
};

export const getE2EStatus = (): E2EStatus | undefined => {
  if (typeof window === 'undefined') {
    return undefined;
  }

  return window.__KUKURI_E2E_STATUS__;
};
