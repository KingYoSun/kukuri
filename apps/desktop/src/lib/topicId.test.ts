import { describe, expect, it } from 'vitest';

import { TOPIC_ID_PREFIX, normalizeTopicId, topicDisplayName } from './topicId';

describe('topicDisplayName', () => {
  it('名前空間 prefix を剥がして表示名を返す', () => {
    expect(topicDisplayName('kukuri:topic:demo')).toBe('demo');
    expect(topicDisplayName(`${TOPIC_ID_PREFIX}iroh`)).toBe('iroh');
  });

  it('prefix の無い ID はそのまま返す', () => {
    expect(topicDisplayName('legacy-topic')).toBe('legacy-topic');
  });

  it('名前部分に : が含まれても先頭 prefix だけを剥がす', () => {
    expect(topicDisplayName('kukuri:topic:a:b')).toBe('a:b');
  });
});

describe('normalizeTopicId', () => {
  it('素の名前へ prefix を付与する', () => {
    expect(normalizeTopicId('demo')).toBe('kukuri:topic:demo');
    expect(normalizeTopicId('  demo  ')).toBe('kukuri:topic:demo');
  });

  it('完全な ID の貼り付けは二重付与しない', () => {
    expect(normalizeTopicId('kukuri:topic:demo')).toBe('kukuri:topic:demo');
  });

  it('空・空白のみの入力は空文字を返す', () => {
    expect(normalizeTopicId('')).toBe('');
    expect(normalizeTopicId('   ')).toBe('');
  });

  it('display 名との往復が安定する', () => {
    expect(normalizeTopicId(topicDisplayName('kukuri:topic:demo'))).toBe('kukuri:topic:demo');
  });
});
