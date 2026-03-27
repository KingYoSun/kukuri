export type DesktopTheme = 'dark' | 'light';

export const DESKTOP_THEME_STORAGE_KEY = 'kukuri.desktop.theme';

export function isDesktopTheme(value: string | null | undefined): value is DesktopTheme {
  return value === 'dark' || value === 'light';
}

export function readDesktopTheme(): DesktopTheme {
  if (typeof window === 'undefined') {
    return 'dark';
  }

  const storedTheme = window.localStorage.getItem(DESKTOP_THEME_STORAGE_KEY);
  return isDesktopTheme(storedTheme) ? storedTheme : 'dark';
}

export function writeDesktopTheme(theme: DesktopTheme) {
  if (typeof window === 'undefined') {
    return;
  }

  window.localStorage.setItem(DESKTOP_THEME_STORAGE_KEY, theme);
}
