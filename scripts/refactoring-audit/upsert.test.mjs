import test from 'node:test';
import assert from 'node:assert/strict';
import { MARKER, BEGIN, END, upsert, githubApi } from './upsert.mjs';

const result = (required = true) => ({
  schema_version: 1, audit_required: required,
  baseline_commit: 'a'.repeat(40), current_commit: 'b'.repeat(40),
  evaluated_at: '2026-09-08T00:00:00Z', tracking_issue: 872,
  force_audit: false, manual_reason: '',
  signals: [{ id: 'ratchet_new_paths', value: 1, threshold: 0, triggered: true, paths: ['a.rs'], reason: 'new path' },
    { id: 'ratchet_increased_caps', value: 0, threshold: 0, triggered: false, paths: [], reason: 'cap growth' }],
  candidate_paths: ['a.rs'], human_only_triggers: ['release'],
  markdown: '## 監査トリガー判定\n\nテストの判定結果\n',
});
const existing = (number = 1) => ({ number, state: 'open', body: `${MARKER}\n人間のscope\n${BEGIN}\nold\n${END}\n人間の判断`, html_url: `https://github.com/o/r/issues/${number}` });
function api(pages = [[]]) {
  const writes = [], reads = [];
  return {
    writes, reads,
    async list(page) { reads.push(page); return pages[page - 1] ?? []; },
    async create(body) { writes.push(['create', body]); return { number: 10, html_url: 'https://github.com/o/r/issues/10' }; },
    async update(number, body) { writes.push(['update', number, body]); return { number, html_url: `https://github.com/o/r/issues/${number}` }; },
  };
}

test('false does not even enter the issue API path', async () => {
  const mock = api();
  const report = result(false);
  report.signals[0] = { ...report.signals[0], value: 0, triggered: false, paths: [] };
  report.candidate_paths = [];
  assert.equal((await upsert(report, mock)).action, 'none');
  assert.deepEqual(mock.writes, []);
  assert.deepEqual(mock.reads, []);
});
test('true creates a finite read-only audit with fixed marker', async () => {
  const mock = api();
  assert.equal((await upsert(result(), mock)).action, 'created');
  assert.equal(mock.writes.length, 1);
  assert.ok(mock.writes[0][1].body.includes(MARKER));
  assert.ok(mock.writes[0][1].body.includes('読み取り中心'));
  assert.ok(mock.writes[0][1].body.includes('#871'));
});
test('update preserves human scope and is idempotent', async () => {
  const issue = existing();
  const mock = api([[issue]]);
  await upsert(result(), mock);
  const body = mock.writes[0][2].body;
  assert.ok(body.startsWith(`${MARKER}\n人間のscope\n${BEGIN}`));
  assert.ok(body.endsWith(`${END}\n人間の判断`));
  issue.body = body;
  await upsert(result(), mock);
  assert.equal(mock.writes.length, 1);
});
test('pagination excludes PRs, unrelated and closed issues', async () => {
  const page = Array.from({ length: 100 }, (_, i) => ({ number: i + 50, state: 'open', body: 'unrelated' }));
  page[0] = { ...existing(40), pull_request: {} };
  page[1] = { ...existing(41), state: 'closed' };
  const mock = api([page, [existing(2)]]);
  await upsert(result(), mock);
  assert.deepEqual(mock.reads, [1, 2]);
  assert.equal(mock.writes[0][1], 2);
});
test('duplicate open marker or malformed management region fails without writes', async () => {
  for (const issues of [[existing(1), existing(2)], [{ ...existing(), body: `${MARKER}\nmissing region` }],
    [{ ...existing(), body: `${MARKER}\n${BEGIN}\n${BEGIN}\n${END}` }]]) {
    const mock = api([issues]);
    await assert.rejects(upsert(result(), mock));
    assert.equal(mock.writes.length, 0);
  }
});
test('lookup failure is not an empty search', async () => {
  const mock = api();
  mock.list = async () => { throw new Error('503'); };
  await assert.rejects(upsert(result(), mock), /503/);
  assert.equal(mock.writes.length, 0);
});
test('lost create response is not retried; next run finds the committed issue', async () => {
  let saved;
  const mock = api();
  mock.list = async () => saved ? [saved] : [];
  mock.create = async ({ body }) => { saved = { ...existing(10), body }; mock.writes.push('create'); throw new Error('response lost'); };
  await assert.rejects(upsert(result(), mock), /response lost/);
  assert.equal((await upsert(result(), mock)).action, 'unchanged');
  assert.deepEqual(mock.writes, ['create']);
});
test('update failure does not fall back to creation', async () => {
  const mock = api([[existing()]]);
  mock.update = async () => { throw new Error('403'); };
  await assert.rejects(upsert(result(), mock), /403/);
  assert.equal(mock.writes.length, 0);
});
test('malformed result and injected management markers fail before API', async () => {
  for (const report of [{}, { ...result(), audit_required: 'true' }, { ...result(), markdown: BEGIN },
    { ...result(), force_audit: true }, { ...result(), current_commit: 'bad' }]) {
    const mock = api();
    await assert.rejects(upsert(report, mock));
    assert.equal(mock.writes.length, 0);
    assert.equal(mock.reads.length, 0);
  }
});
test('REST adapter uses JSON data, pagination and no automatic write retry', async () => {
  const calls = [];
  const fetcher = async (url, options) => {
    calls.push([url, options]);
    return { ok: true, status: 200, json: async () => options.method === 'GET' ? [] : { number: 8, html_url: 'https://github.com/o/r/issues/8' } };
  };
  const adapter = githubApi('o/r', 'test-token', fetcher);
  await adapter.list(2);
  await adapter.create({ title: 'audit', body: '$(touch nope)\n`code`' });
  assert.ok(calls[0][0].endsWith('state=open&per_page=100&page=2'));
  assert.equal(JSON.parse(calls[1][1].body).body, '$(touch nope)\n`code`');
  assert.equal(calls[1][1].headers.Authorization, 'Bearer test-token');
  let failedCalls = 0;
  const failed = githubApi('o/r', 'test-token', async () => { failedCalls++; return { ok: false, status: 503 }; });
  await assert.rejects(failed.create({}), /503/);
  assert.equal(failedCalls, 1);
});
