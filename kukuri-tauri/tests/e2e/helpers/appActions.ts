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
  try {
    await home.waitForDisplayed();
  } catch {
    const debugSnapshot = await browser.execute(() => {
      return {
        location: window.location.pathname,
        auth: document.documentElement?.getAttribute('data-e2e-auth') ?? null,
        lastLog: document.documentElement?.getAttribute('data-e2e-last-log') ?? null,
      };
    });
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
