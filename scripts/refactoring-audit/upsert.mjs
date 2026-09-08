import { readFile, appendFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

export const MARKER = '<!-- kukuri-refactoring-audit:v1 -->';
export const BEGIN = '<!-- kukuri-refactoring-audit:latest:start -->';
export const END = '<!-- kukuri-refactoring-audit:latest:end -->';

function require(value, message) {
  if (!value) throw new Error(message);
}

export function validateResult(result) {
  require(result?.schema_version === 1 && typeof result.audit_required === 'boolean', 'invalid audit result');
  require(/^[a-f0-9]{40}$/.test(result.baseline_commit) && /^[a-f0-9]{40}$/.test(result.current_commit), 'invalid commit');
  require(typeof result.evaluated_at === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/.test(result.evaluated_at)
    && Number.isFinite(Date.parse(result.evaluated_at)), 'invalid evaluation time');
  require(Number.isSafeInteger(result.tracking_issue) && result.tracking_issue > 0, 'invalid tracking issue');
  require(typeof result.force_audit === 'boolean' && typeof result.manual_reason === 'string', 'invalid manual input');
  require(!result.force_audit || result.manual_reason.trim(), 'force reason required');
  require(Array.isArray(result.signals) && result.signals.length === 2, 'invalid signals');
  const names = ['ratchet_new_paths', 'ratchet_increased_caps'];
  for (const [index, signal] of result.signals.entries()) {
    require(signal.id === names[index] && Number.isSafeInteger(signal.value) && signal.value >= 0
      && signal.threshold === 0 && signal.triggered === (signal.value > 0)
      && Array.isArray(signal.paths) && signal.paths.length === signal.value
      && signal.paths.every(path => typeof path === 'string') && new Set(signal.paths).size === signal.paths.length,
    'invalid signal value');
  }
  require(result.audit_required === (result.force_audit || result.signals.some(signal => signal.triggered)), 'inconsistent audit_required');
  require(Array.isArray(result.candidate_paths) && result.candidate_paths.every(path => typeof path === 'string'), 'invalid candidate paths');
  require(Array.isArray(result.human_only_triggers) && result.human_only_triggers.every(item => typeof item === 'string'), 'invalid human triggers');
  require(typeof result.markdown === 'string' && result.markdown.length > 0
    && ![MARKER, BEGIN, END].some(marker => result.markdown.includes(marker)), 'invalid markdown/management marker');
}

function escape(text) {
  return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('\r', '').replaceAll('\n', '<br>');
}

function region(result) {
  return `${BEGIN}\n${result.markdown.trimEnd()}\n${END}`;
}

function newIssue(result) {
  const scope = result.candidate_paths.map(path => `- ${escape(path)}`).join('\n') || '- 手動理由を確認して担当者が対象path／責務を固定する。';
  return {
    title: `リファクタリング監査: ${result.evaluated_at.slice(0, 10)}のtrigger判定`,
    body: `${MARKER}
## Current status
- 判定: Planned（読み取り中心の監査から開始）
- Scope revision: audit-${result.baseline_commit.slice(0, 12)}
- In scope: 下記の開始時候補から担当者が有限scopeを固定する。
- Non-goals: 自動実装、直接PR作成、候補の無条件採用。
- 所有: 本Issueは監査と候補分類を所有し、実装は必要な個別Issueへ分ける。

## 対象と開始trigger
規範は #871 と REFACTORING.md、直前の監査／campaignは #${result.tracking_issue}。
開始理由: ${escape(result.manual_reason) || 'ratchetの増加を観測したため。'}

## 有限scopeと終了条件
以下は開始時の候補であり、以後の自動判定はこのscopeを変更しない。
${scope}

担当者が対象path／責務と確認集合または時間枠を固定してから読み取り中心の監査を行う。
- [ ] AC-1: 固定したscopeの全候補を、実施／延期／却下／別種別へ根拠付きで分類する。
- [ ] AC-2: 実施候補は一つの構造上の成果ごとに個別Issueへ渡す。変更しない結論も正常な完了とする。
- [ ] INVAR-1: この監査ではproduct code、baseline、製品契約を自動変更しない。
Issue lifecycleのリスク区分と必要なinventory／transitionはscope確定時に担当者が記録する。

## 変更前baseline
比較起点: ${result.baseline_commit}、開始時評価commit: ${result.current_commit}。

## 候補一覧
| 候補 | 観測した問題と根拠 | 目標成果 | 優先順位 | 分類 | 理由 / 個別Issue |
| --- | --- | --- | --- | --- | --- |

## 監査終了判定
担当者が固定scopeの分類結果と終了根拠を記録する。baseline更新は監査完了後のreview付きPRで行う。

## 最新の機械観測（上記scopeを拡張しない）
${region(result)}
`,
  };
}

function replaceRegion(body, result) {
  require(body.split(BEGIN).length === 2 && body.split(END).length === 2, 'ambiguous or missing managed region');
  const start = body.indexOf(BEGIN), end = body.indexOf(END);
  require(start < end, 'invalid managed region order');
  return body.slice(0, start) + region(result) + body.slice(end + END.length);
}

/** Caller must serialize all runs using one repository-wide Actions concurrency group. */
export async function upsert(result, api) {
  validateResult(result);
  if (!result.audit_required) return { action: 'none' };
  const matches = [];
  for (let page = 1; ; page++) {
    const issues = await api.list(page);
    require(Array.isArray(issues), 'invalid issue listing');
    for (const issue of issues) {
      if (issue.state === 'open' && !issue.pull_request && typeof issue.body === 'string'
        && issue.body.split(/\r?\n/).includes(MARKER)) matches.push(issue);
    }
    if (issues.length < 100) break;
  }
  require(matches.length <= 1, 'multiple open audit issues: resolve markers manually; no writes made');
  if (matches.length === 0) {
    const payload = newIssue(result);
    require(payload.body.length <= 65000, 'audit body too large; no writes made');
    const created = await api.create(payload);
    return { action: 'created', number: created.number, url: created.html_url };
  }
  const issue = matches[0];
  require(Number.isSafeInteger(issue.number) && issue.number > 0, 'invalid issue number');
  const body = replaceRegion(issue.body, result);
  require(body.length <= 65000, 'audit body too large; no writes made');
  if (body === issue.body) return { action: 'unchanged', number: issue.number, url: issue.html_url };
  await api.update(issue.number, { body });
  return { action: 'updated', number: issue.number, url: issue.html_url };
}

export function githubApi(repository, token, fetcher = fetch) {
  require(/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repository) && token, 'repository and token required');
  const base = `https://api.github.com/repos/${repository}/issues`;
  async function request(method, suffix = '', body) {
    // No shell, search query, automatic retry, or redirects for token-bearing requests.
    const response = await fetcher(base + suffix, {
      method, redirect: 'error', signal: AbortSignal.timeout(30000),
      headers: { Authorization: `Bearer ${token}`, Accept: 'application/vnd.github+json',
        'X-GitHub-Api-Version': '2022-11-28', 'Content-Type': 'application/json' },
      ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    });
    require(response.ok, `GitHub ${method} failed (${response.status}); inspect before rerun`);
    return response.json();
  }
  return {
    list: page => request('GET', `?state=open&per_page=100&page=${page}`),
    create: payload => request('POST', '', payload),
    update: (number, payload) => request('PATCH', `/${number}`, payload),
  };
}

async function main() {
  const result = JSON.parse(await readFile(process.argv[2], 'utf8'));
  const outcome = await upsert(result, githubApi(process.env.GITHUB_REPOSITORY, process.env.GH_TOKEN));
  process.stdout.write(`${JSON.stringify(outcome)}\n`);
  if (process.env.GITHUB_STEP_SUMMARY) {
    await appendFile(process.env.GITHUB_STEP_SUMMARY, `\nIssue: ${outcome.action}${outcome.number ? ` #${outcome.number}` : ''}\n`);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch(error => { process.stderr.write(`refactoring_audit_upsert_error: ${error.message}\n`); process.exitCode = 1; });
}
