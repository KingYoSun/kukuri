import { $, browser } from '@wdio/globals';

export async function waitForAppReady(timeout = 20000): Promise<void> {
  await browser.waitUntil(
    async () => {
      const root = await $('#root');
      const exists = await root.isExisting();
      if (!exists) {
        return false;
      }
      const isDisplayed = await root.isDisplayed();
      return isDisplayed;
    },
    {
      timeout,
      interval: 500,
      timeoutMsg: 'Timed out waiting for Kukuri root container'
    }
  );
}
