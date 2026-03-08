import { $, browser } from '@wdio/globals';

export interface ProfileInfo {
  name: string;
  displayName: string;
  about: string;
}

async function setControlledInputValue(selector: string, value: string): Promise<void> {
  const input = await $(selector);
  await input.waitForDisplayed({ timeout: 15000 });
  await input.clearValue();
  await input.setValue(value);
  await browser.waitUntil(async () => (await input.getValue()) === value, {
    timeout: 10000,
    interval: 200,
    timeoutMsg: `Input ${selector} did not retain expected value`,
  });
}

export async function waitForWelcome(): Promise<void> {
  const welcome = await $('[data-testid="welcome-screen"]');
  await welcome.waitForDisplayed();
}

export async function startCreateAccountFlow(): Promise<void> {
  const createButton = await $('[data-testid="welcome-create-account"]');
  await createButton.waitForDisplayed({ timeout: 15000 });
  await createButton.waitForClickable({ timeout: 15000 });
  await createButton.scrollIntoView();
  await createButton.click();

  try {
    await browser.waitUntil(
      async () => {
        const pathname = await browser.execute(() => window.location.pathname);
        if (pathname === '/profile-setup') {
          return true;
        }
        const form = await $('[data-testid="profile-form"]');
        return await form.isExisting();
      },
      {
        timeout: 30000,
        interval: 300,
        timeoutMsg: 'Profile setup route did not activate',
      },
    );
  } catch (error) {
    const debugSnapshot = await browser.execute(() => {
      const doc = document.documentElement;
      return {
        location: window.location.pathname,
        auth: doc?.getAttribute('data-e2e-auth') ?? null,
        lastLog: doc?.getAttribute('data-e2e-last-log') ?? null,
        pageError: doc?.getAttribute('data-kukuri-e2e-error') ?? null,
        createButtonDisabled:
          document
            .querySelector('[data-testid="welcome-create-account"]')
            ?.getAttribute('disabled') !== null,
        authSnapshot: window.__KUKURI_E2E__?.getAuthSnapshot?.() ?? null,
      };
    });
    throw new Error(
      `Failed to start create-account flow: ${
        error instanceof Error ? error.message : String(error)
      }; snapshot=${JSON.stringify(debugSnapshot)}`,
      {
        cause: error,
      },
    );
  }
}

export async function completeProfileSetup(profile: ProfileInfo): Promise<void> {
  const form = await $('[data-testid="profile-form"]');
  try {
    await form.waitForDisplayed();
  } catch {
    const debugSnapshot = await browser.execute(() => {
      return {
        location: window.location.pathname,
        auth: document.documentElement?.getAttribute('data-e2e-auth') ?? null,
        lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
        pageError: document.documentElement?.getAttribute('data-kukuri-e2e-error') ?? null,
      };
    });
    throw new Error(
      `profile-form not visible (location=${debugSnapshot.location}, auth=${debugSnapshot.auth}, lastLog=${debugSnapshot.lastLog}, pageError=${debugSnapshot.pageError})`,
    );
  }

  await setControlledInputValue('[data-testid="profile-name"]', profile.name);
  await setControlledInputValue('[data-testid="profile-display-name"]', profile.displayName);
  await setControlledInputValue('[data-testid="profile-about"]', profile.about);
  await $('[data-testid="profile-submit"]').click();
}

export async function waitForHome(): Promise<void> {
  const home = await $('[data-testid="home-page"]');
  try {
    await home.waitForDisplayed({ timeout: 20000 });
  } catch {
    const debugSnapshot = await browser.execute(() => {
      return {
        location: window.location.pathname,
        auth: document.documentElement?.getAttribute('data-e2e-auth') ?? null,
        lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
        pageError: document.documentElement?.getAttribute('data-kukuri-e2e-error') ?? null,
      };
    });
    throw new Error(
      `home-page not visible (location=${debugSnapshot.location}, auth=${debugSnapshot.auth}, lastLog=${debugSnapshot.lastLog}, pageError=${debugSnapshot.pageError})`,
    );
  }
}

export async function openSettings(): Promise<void> {
  const openButton = () => $('[data-testid="open-settings-button"]');
  const settingsPage = () => $('[data-testid="settings-page"]');

  const triggerNavigation = async () => {
    const button = await openButton();
    await button.waitForClickable({ timeout: 7000 });
    await button.scrollIntoView();
    await button.click();
  };

  let lastError: unknown = null;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await triggerNavigation();
      await browser.waitUntil(
        async () => {
          const pathname = await browser.execute(() => window.location.pathname);
          return typeof pathname === 'string' && pathname.startsWith('/settings');
        },
        { timeout: 15000, interval: 300, timeoutMsg: 'Settings route did not activate' },
      );
      await (await settingsPage()).waitForDisplayed({ timeout: 20000 });
      return;
    } catch (error) {
      lastError = error;
      await browser.pause(500);
    }
  }

  throw new Error(
    `settings-page not visible after user navigation attempts (lastError=${
      lastError instanceof Error ? lastError.message : String(lastError)
    })`,
  );
}

export async function openAccountMenu(): Promise<void> {
  const menu = () => $('[data-testid="account-menu-go-login"]');

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      const trigger = await $('[data-testid="account-switcher-trigger"]');
      await trigger.waitForClickable({ timeout: 7000 });
      await trigger.scrollIntoView();
      await trigger.click();
    } catch {
      if (attempt === 2) {
        throw new Error('Account menu trigger was not clickable');
      }
    }
    try {
      await (await menu()).waitForDisplayed({ timeout: 7000 });
      return;
    } catch {
      if (attempt === 2) {
        throw new Error('Account menu did not open after multiple attempts');
      }
      await browser.pause(500);
    }
  }
}
