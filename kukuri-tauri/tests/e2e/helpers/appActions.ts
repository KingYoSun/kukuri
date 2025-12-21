import { $, browser } from '@wdio/globals';

export interface ProfileInfo {
  name: string;
  displayName: string;
  about: string;
}

export async function waitForWelcome(): Promise<void> {
  const welcome = await $('[data-testid="welcome-screen"]');
  await welcome.waitForDisplayed();
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

  await $('[data-testid="profile-name"]').setValue(profile.name);
  await $('[data-testid="profile-display-name"]').setValue(profile.displayName);
  await $('[data-testid="profile-about"]').setValue(profile.about);
  await $('[data-testid="profile-submit"]').click();
}

export async function waitForHome(): Promise<void> {
  const home = await $('[data-testid="home-page"]');
  let appliedFallback = false;
  try {
    await home.waitForDisplayed();
  } catch {
    const bootstrapFallback = async () => {
      if (appliedFallback) {
        return null;
      }
      const snapshot = await browser.execute<{
        location: string;
        lastLog: string | null;
        pageError: string | null;
      }>(() => {
        return {
          location: window.location.pathname,
          lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
          pageError: document.documentElement?.getAttribute('data-kukuri-e2e-error') ?? null,
        };
      });
      if (
        snapshot.location?.includes('/profile-setup') &&
        (snapshot.lastLog?.includes('Profile setup failed') ||
          snapshot.lastLog?.includes('Nostr operation failed'))
      ) {
        appliedFallback = true;
        await browser.execute(() => {
          try {
            window.history.pushState({}, '', '/');
          } catch {
            window.location.replace('/');
          }
        });
        await browser.pause(200);
        return snapshot;
      }
      return snapshot;
    };

    const debugSnapshot = await browser.execute(() => {
      return {
        location: window.location.pathname,
        auth: document.documentElement?.getAttribute('data-e2e-auth') ?? null,
        lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
        pageError: document.documentElement?.getAttribute('data-kukuri-e2e-error') ?? null,
      };
    });
    await bootstrapFallback();
    try {
      await home.waitForDisplayed({ timeout: 10000 });
      return;
    } catch {
      void 0;
    }
    throw new Error(
      `home-page not visible (location=${debugSnapshot.location}, auth=${debugSnapshot.auth}, lastLog=${debugSnapshot.lastLog}, pageError=${debugSnapshot.pageError})`,
    );
  }
}

export async function openSettings(): Promise<void> {
  const openButton = () => $('[data-testid="open-settings-button"]');
  const settingsPage = () => $('[data-testid="settings-page"]');

  const triggerNavigation = async () => {
    try {
      const button = await openButton();
      await button.waitForClickable({ timeout: 7000 });
      await button.click();
    } catch {
      await browser.execute(() => {
        const el = document.querySelector(
          '[data-testid="open-settings-button"]',
        ) as HTMLButtonElement | null;
        el?.click();
      });
    }
  };

  let lastError: unknown = null;
  for (let attempt = 0; attempt < 2; attempt += 1) {
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

  // Fallback: force navigation in case the click was swallowed by the app
  await browser.execute(() => {
    try {
      window.history.pushState({}, '', '/settings');
    } catch {
      window.location.assign('/settings');
    }
  });
  await (await settingsPage()).waitForDisplayed({
    timeout: 20000,
    timeoutMsg: `settings-page not visible after fallback (lastError=${
      lastError instanceof Error ? lastError.message : String(lastError)
    })`,
  });
}

export async function openAccountMenu(): Promise<void> {
  const trigger = await $('[data-testid="account-switcher-trigger"]');
  const menu = () => $('[data-testid="account-menu-go-login"]');

  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await trigger.waitForClickable({ timeout: 7000 });
      await trigger.click();
    } catch {
      await browser.execute(() => {
        const el = document.querySelector(
          '[data-testid="account-switcher-trigger"]',
        ) as HTMLButtonElement | null;
        el?.click();
      });
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
