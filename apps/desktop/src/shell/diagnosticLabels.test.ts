import { createInstance } from 'i18next';
import { expect, test } from 'vitest';

import { resources } from '@/i18n';
import { diagnosticErrorLabel, diagnosticStatusDetail, diagnosticValueLabel } from './diagnosticLabels';

test.each([
  ['ja', 'トピックへの初回参加がタイムアウトしました', '直接 P2P', 'リレー補助 P2P', 'リレー経由の代替接続'],
  ['en', 'The initial topic join timed out', 'Direct P2P', 'Relay-supported P2P', 'Relay fallback'],
  ['zh-CN', '首次加入话题超时', '直接 P2P', '中继辅助 P2P', '中继备用连接'],
])('diagnostic copy in %s separates paths and keeps the original error', async (locale, timeout, direct, supported, fallback) => {
  const i18n = createInstance();
  await i18n.init({ resources, lng: locale, fallbackLng: 'en', interpolation: { escapeValue: false } });
  const t = i18n.t.bind(i18n);
  expect(diagnosticValueLabel('path', 'direct_p2p', t, false)).toBe(direct);
  expect(diagnosticValueLabel('path', 'relay_supported_p2p', t, false)).toBe(supported);
  expect(diagnosticValueLabel('path', 'relay_fallback', t, false)).toBe(fallback);
  for (const raw of ['timed out waiting for initial topic join', 'topic join pending: timed out waiting for initial topic join']) {
    expect(diagnosticErrorLabel(raw, t)).toContain(timeout);
    expect(diagnosticErrorLabel(raw, t)).toContain(raw);
    expect(diagnosticStatusDetail(raw, t)).toBe(diagnosticErrorLabel(raw, t));
  }
  for (const raw of ['not timed out waiting for initial topic join', 'upload failed: timed out waiting for initial topic join', '未知のエラー <node>&']) {
    expect(diagnosticErrorLabel(raw, t)).not.toContain(timeout);
    expect(diagnosticErrorLabel(raw, t)).toContain(raw);
  }
  expect(diagnosticErrorLabel(null, t)).toBeNull();
  expect(diagnosticValueLabel('path', 'future_path', t, false)).not.toContain(direct);
  expect(diagnosticValueLabel('path', 'future_path', t, false)).toContain('future_path');
  expect(diagnosticValueLabel('capability', 'constructor', t)).not.toContain('settings:');
});
