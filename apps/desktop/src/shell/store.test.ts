import { beforeEach, expect, test } from 'vitest';

import { createInitialShellState } from '@/shell/store';

beforeEach(() => {
  window.history.replaceState(null, '', '/');
});

test('shell store keeps settings drawer closed by default', () => {
  const state = createInitialShellState();

  expect(state.shellChromeState.activeSettingsSection).toBe('connectivity');
  expect(state.shellChromeState.settingsOpen).toBe(false);
});

test('shell store seeds settings drawer state from the initial hash route', () => {
  window.history.replaceState(
    null,
    '',
    '/#/timeline?topic=kukuri%3Atopic%3Ademo&settings=appearance'
  );

  const state = createInitialShellState();

  expect(state.shellChromeState.activeSettingsSection).toBe('appearance');
  expect(state.shellChromeState.settingsOpen).toBe(true);
});
