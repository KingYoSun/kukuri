import { $, browser, expect } from '@wdio/globals';
import { waitForAppReady } from '../helpers/waitForAppReady.ts';

describe('Kukuri Desktop App', () => {
  before(async () => {
    await waitForAppReady();
  });

  it('renders the root container', async () => {
    const root = await $('#root');
    await expect(root).toBeExisting();
    await expect(root).toBeDisplayed();
  });

  it('exposes the Kukuri window title', async () => {
    const title = await browser.getTitle();
    expect(title.toLowerCase()).toContain('kukuri');
  });
});
