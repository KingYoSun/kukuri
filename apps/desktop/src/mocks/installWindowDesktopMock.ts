import { createDesktopMockApi, type DesktopMockApiOptions } from '@/mocks/desktopApiMock';

export function installWindowDesktopMock(options?: DesktopMockApiOptions) {
  window.__KUKURI_DESKTOP__ = createDesktopMockApi(options);
  return window.__KUKURI_DESKTOP__;
}
