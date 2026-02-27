export const NETWORK_STATUS_REFRESH_EVENT = 'kukuri:network-status-refresh';

export const emitNetworkStatusRefresh = () => {
  if (typeof window === 'undefined') {
    return;
  }
  window.dispatchEvent(new Event(NETWORK_STATUS_REFRESH_EVENT));
};

export const subscribeNetworkStatusRefresh = (handler: () => void): (() => void) => {
  if (typeof window === 'undefined') {
    return () => {};
  }

  const listener = () => {
    handler();
  };

  window.addEventListener(NETWORK_STATUS_REFRESH_EVENT, listener);
  return () => {
    window.removeEventListener(NETWORK_STATUS_REFRESH_EVENT, listener);
  };
};
