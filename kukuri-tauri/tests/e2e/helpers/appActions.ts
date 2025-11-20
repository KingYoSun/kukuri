import { $ } from '@wdio/globals';

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
  await form.waitForDisplayed();

  await $('[data-testid="profile-name"]').setValue(profile.name);
  await $('[data-testid="profile-display-name"]').setValue(profile.displayName);
  await $('[data-testid="profile-about"]').setValue(profile.about);
  await $('[data-testid="profile-submit"]').click();
}

export async function waitForHome(): Promise<void> {
  const home = await $('[data-testid="home-page"]');
  await home.waitForDisplayed();
}

export async function openSettings(): Promise<void> {
  await $('[data-testid="open-settings-button"]').click();
  await $('[data-testid="settings-page"]').waitForDisplayed();
}

export async function openAccountMenu(): Promise<void> {
  await $('[data-testid="account-switcher-trigger"]').click();
  await $('[data-testid="account-menu-go-login"]').waitForDisplayed();
}
