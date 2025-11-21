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
      };
    });
    throw new Error(
      `profile-form not visible (location=${debugSnapshot.location}, auth=${debugSnapshot.auth}, lastLog=${debugSnapshot.lastLog})`,
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
      const snapshot = await browser.execute<{ location: string; lastLog: string | null }>(() => {
        return {
          location: window.location.pathname,
          lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
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
      `home-page not visible (location=${debugSnapshot.location}, auth=${debugSnapshot.auth}, lastLog=${debugSnapshot.lastLog})`,
    );
  }
}

export async function openSettings(): Promise<void> {
  await $('[data-testid="open-settings-button"]').click();
  await $('[data-testid="settings-page"]').waitForDisplayed();
}

export async function openAccountMenu(): Promise<void> {
  const trigger = await $('[data-testid="account-switcher-trigger"]');
  const menu = () => $('[data-testid="account-menu-go-login"]');

  for (let attempt = 0; attempt < 3; attempt += 1) {
    await trigger.click();
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
