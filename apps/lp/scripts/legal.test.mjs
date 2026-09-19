import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import test from 'node:test';

test('legal pages match the canonical client documents', () => {
  const result = spawnSync(process.execPath, ['apps/lp/scripts/sync-legal.mjs', '--check'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

for (const [route, source] of [['privacy', 'privacy-policy.md'], ['terms', 'terms-of-service.md']]) {
  test(`${route} exposes the complete canonical text without JavaScript`, () => {
    const html = readFileSync(`apps/lp/public/${route}/index.html`, 'utf8');
    const article = html.match(/<article>([\s\S]*?)<\/article>/)[1];
    const actual = article.replace(/<[^>]*>/g, '').replaceAll('&quot;', '"').replaceAll('&lt;', '<').replaceAll('&gt;', '>').replaceAll('&amp;', '&');
    const expected = readFileSync(`docs/legal/${source}`, 'utf8')
      .replace(/^\d+\. /gm, '').replace(/^#{1,3} /gm, '').replace(/^> /gm, '').replace(/^- /gm, '').replace(/`/g, '').replace(/\*\*/g, '');
    assert.equal(actual.replace(/\s/g, ''), expected.replace(/\s/g, ''));
    assert.doesNotMatch(html, /<script\b/i);
    assert.match(html, /<html lang="ja">/);
  });
}

for (const locale of ['', 'en/']) {
  test(`${locale || 'ja'} LP links client terms and privacy, labels Node-only links`, () => {
    const html = readFileSync(`apps/lp/public/${locale}index.html`, 'utf8');
    assert.match(html, /href="\/terms\/"/);
    assert.match(html, /href="\/privacy\/"/);
    assert.doesNotMatch(html, /href="https:\/\/api\.kukuri\.app\/(terms|privacy)"/);
    assert.match(html, /Community Node/);
  });
}
