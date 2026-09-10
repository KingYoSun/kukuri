import { renderHook } from '@testing-library/react';
import { createInstance } from 'i18next';
import { expect, test } from 'vitest';

import { resources, type SupportedLocale } from '@/i18n';
import { createDesktopShellStore } from '@/shell/store';
import { useSettingsViewModels } from './useSettingsViewModels';

async function setup(locale: SupportedLocale = 'ja') {
  const i18n = createInstance();
  await i18n.init({ resources, lng: locale, fallbackLng: 'en' });
  const state = createDesktopShellStore().getState();
  const error = 'topic join pending: timed out waiting for initial topic join';
  const args: Parameters<typeof useSettingsViewModels>[0] = {
    ...state, locale, theme: 'dark', t: i18n.t.bind(i18n), trackedTopics: [], topicDiagnostics: {},
    syncStatus: {
      ...state.syncStatus, last_error: error, status_detail: error,
      discovery: { ...state.syncStatus.discovery, mode: 'seeded_dht', connect_mode: 'direct_or_relay', last_discovery_error: error },
    },
  };
  return { args, i18n, error };
}

test('Japanese diagnostics label discovery values and summarize the reported timeout', async () => {
  const { args, error } = await setup();
  const before = JSON.stringify(args.syncStatus);
  const { result, rerender } = renderHook(useSettingsViewModels, { initialProps: args });
  expect(result.current.discoveryPanelView.summaryLabel).toContain('シード付き DHT');
  expect(result.current.discoveryPanelView.metrics[1].value).toContain('直接接続またはリレー');
  const displayedErrors = [
    result.current.connectivityPanelView.diagnostics[1].value,
    result.current.connectivityPanelView.diagnostics[3].value,
    result.current.discoveryPanelView.diagnostics.at(-1)!.value,
  ];
  for (const value of displayedErrors) {
    expect(value).toContain('トピックへの初回参加がタイムアウトしました');
    expect(value).toContain(error);
  }
  expect(JSON.stringify(args.syncStatus)).toBe(before);
  rerender({ ...args, syncStatus: { ...args.syncStatus, last_error: null, discovery: { ...args.syncStatus.discovery, last_discovery_error: null } } });
  expect(result.current.connectivityPanelView.diagnostics[3].value).toBe('なし');
  expect(result.current.discoveryPanelView.diagnostics.at(-1)!.value).toBe('なし');
});

test('unknown diagnostic errors retain raw evidence under a Japanese summary', async () => {
  const { args } = await setup();
  const raw = 'unrecognized upstream failure';
  args.syncStatus = { ...args.syncStatus, last_error: raw };
  const { result } = renderHook(useSettingsViewModels, { initialProps: args });
  const value = result.current.connectivityPanelView.diagnostics[3].value;
  expect(value).toContain('エラーが発生しました');
  expect(value).toContain(raw);
});
