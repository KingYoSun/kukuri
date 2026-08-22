import { type ComponentProps } from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { CommunityNodeManifest, DesktopApi } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

const NODE_A = 'https://index-a.example';
const NODE_B = 'https://index-b.example';

function manifestFor(baseUrl: string, nodeId: string): CommunityNodeManifest {
  const host = new URL(baseUrl).host;
  return {
    node_id: nodeId,
    node_name: `Index node ${nodeId}`,
    node_role: 'community-node',
    server_name: host,
    manifest_version: 'v1',
    capability_scope: { available_enabled: ['community_index'], planned_enabled: [] },
    authority_scope: {
      applies_to: ['this_node', 'communities_indexed_by_this_node'],
      does_not_apply_to: [],
    },
    p2p_boundary: {
      identity_authority: false,
      profile_canonical_store: false,
      social_graph_canonical_store: false,
      content_truth_source: false,
      network_wide_authority: false,
    },
    abuse_contact: `abuse@${host}`,
    report_endpoint: `${baseUrl}/v1/report`,
    terms_url: '',
    privacy_url: '',
    moderation_policy_url: '',
  };
}

const manifest = manifestFor(NODE_A, 'node-a');

function indexEntry(objectId: string, text: string) {
  return {
    scope_kind: 'public_topic' as const,
    scope_id: 'rust',
    object_id: objectId,
    author_pubkey: `author-${objectId}`,
    text,
    created_at: 42,
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

function workspaceProps(
  api: DesktopApi,
  overrides: Partial<ComponentProps<typeof CommunityIndexWorkspace>> = {}
): ComponentProps<typeof CommunityIndexWorkspace> {
  return {
    api,
    mode: 'topic',
    locale: 'en',
    activeTopic: 'rust',
    activeTimelineScope: { kind: 'public' },
    eligibleNodeBaseUrls: [NODE_A, NODE_B],
    selectedNodeBaseUrl: NODE_A,
    onSelectNode: vi.fn(),
    onOpenCommunityNodeSettings: vi.fn(),
    ...overrides,
  };
}

function runSearch(query = 'hello') {
  fireEvent.change(screen.getByLabelText('Search query'), { target: { value: query } });
  fireEvent.click(screen.getByRole('button', { name: 'Run' }));
}

test('query node selection uses the shared semantic select surface', () => {
  const api = {} as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  expect(screen.getByLabelText('Query node')).toHaveClass(
    'bg-[var(--surface-input)]',
    'text-foreground'
  );
});

test('topic search always sends the active public scope and labels preview text', async () => {
  const searchCommunityNodeIndex = vi.fn().mockResolvedValue({
    entries: [
      {
        scope_kind: 'public_topic',
        scope_id: 'rust',
        object_id: 'post-1',
        author_pubkey: 'author-1',
        text: 'hello\nderived-tag',
        created_at: 42,
      },
    ],
  });
  const api = { searchCommunityNodeIndex } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  runSearch();

  await waitFor(() => expect(searchCommunityNodeIndex).toHaveBeenCalledTimes(1));
  expect(searchCommunityNodeIndex).toHaveBeenCalledWith(
    expect.objectContaining({
      scope_kind: 'public_topic',
      scope_id: 'rust',
      query: 'hello',
    })
  );
  expect(await screen.findByText(/Search preview; may include derived tags/)).toBeInTheDocument();
});

test('all joined topic scope is disabled without sending a query', () => {
  const api = { searchCommunityNodeIndex: vi.fn() } as unknown as DesktopApi;
  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { activeTimelineScope: { kind: 'all_joined' } })}
    />
  );
  expect(screen.getByText(/Search one public topic or private channel at a time/)).toBeInTheDocument();
  expect(screen.queryByLabelText('Search query')).not.toBeInTheDocument();
});

test('changing the selected node clears results and prevents reporting them to the new node', async () => {
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({
      entries: [indexEntry('post-a', 'result from node A')],
    }),
  } as unknown as DesktopApi;
  const props = workspaceProps(api);
  const { rerender } = render(<CommunityIndexWorkspace {...props} />);

  runSearch();
  expect(await screen.findByText('result from node A')).toBeInTheDocument();

  rerender(<CommunityIndexWorkspace {...props} selectedNodeBaseUrl={NODE_B} />);

  await waitFor(() => expect(screen.queryByText('result from node A')).not.toBeInTheDocument());
  expect(screen.queryByRole('button', { name: 'Report' })).not.toBeInTheDocument();
});

test('a pending response is discarded when its node or scope is no longer active', async () => {
  const pending = deferred<{ entries: ReturnType<typeof indexEntry>[] }>();
  const api = {
    searchCommunityNodeIndex: vi.fn().mockReturnValue(pending.promise),
  } as unknown as DesktopApi;
  const props = workspaceProps(api);
  const { rerender } = render(<CommunityIndexWorkspace {...props} />);

  runSearch();
  rerender(
    <CommunityIndexWorkspace
      {...props}
      selectedNodeBaseUrl={NODE_B}
      activeTimelineScope={{ kind: 'channel', channel_id: 'private-1' }}
    />
  );
  pending.resolve({ entries: [indexEntry('post-a', 'late result from node A')] });

  await waitFor(() => expect(api.searchCommunityNodeIndex).toHaveBeenCalledTimes(1));
  expect(screen.queryByText('late result from node A')).not.toBeInTheDocument();
});

test('responses that complete in reverse order keep only the current request context', async () => {
  const responseA = deferred<{ entries: ReturnType<typeof indexEntry>[] }>();
  const responseB = deferred<{ entries: ReturnType<typeof indexEntry>[] }>();
  const searchCommunityNodeIndex = vi.fn((request: { base_url: string }) =>
    request.base_url === NODE_A ? responseA.promise : responseB.promise
  );
  const api = { searchCommunityNodeIndex } as unknown as DesktopApi;
  const props = workspaceProps(api);
  const { rerender } = render(<CommunityIndexWorkspace {...props} />);

  runSearch('node A');
  rerender(<CommunityIndexWorkspace {...props} selectedNodeBaseUrl={NODE_B} />);
  runSearch('node B');
  responseB.resolve({ entries: [indexEntry('post-b', 'current result from node B')] });
  expect(await screen.findByText('current result from node B')).toBeInTheDocument();

  responseA.resolve({ entries: [indexEntry('post-a', 'stale result from node A')] });
  await waitFor(() => expect(searchCommunityNodeIndex).toHaveBeenCalledTimes(2));
  expect(screen.queryByText('stale result from node A')).not.toBeInTheDocument();
  expect(screen.getByText('current result from node B')).toBeInTheDocument();
});

test('recommendation reports use the source node latest manifest and recommendation identity', async () => {
  const freshManifest = {
    ...manifest,
    node_id: 'node-a-fresh',
    report_endpoint: `${NODE_A}/v2/report`,
  };
  const fetchCommunityNodeManifest = vi.fn().mockResolvedValue({
    status: 'ok',
    manifest: freshManifest,
  });
  const submitCommunityNodeReport = vi.fn().mockResolvedValue({
    accepted: true,
    reference_id: 'report-1',
  });
  const api = {
    recommendCommunityNodeIndex: vi.fn().mockResolvedValue({
      entries: [indexEntry('recommendation-1', 'recommended result')],
    }),
    fetchCommunityNodeManifest,
    submitCommunityNodeReport,
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api, { mode: 'explore' })} />);

  fireEvent.click(screen.getByRole('tab', { name: 'Recommendations' }));
  fireEvent.click(screen.getByRole('button', { name: 'Run' }));
  expect(await screen.findByText('recommended result')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Report' }));

  const dialog = await screen.findByRole('dialog', { name: 'Report content' });
  await waitFor(() => expect(fetchCommunityNodeManifest).toHaveBeenCalledWith(NODE_A));
  expect(within(dialog).getByText(/^Recommendation · rust$/)).toBeInTheDocument();
  expect(within(dialog).getByText('Recommendation')).toBeInTheDocument();
  fireEvent.click(await within(dialog).findByRole('button', { name: 'Send report' }));

  await waitFor(() =>
    expect(submitCommunityNodeReport).toHaveBeenCalledWith(
      expect.objectContaining({
        node_base_url: NODE_A,
        report_endpoint: `${NODE_A}/v2/report`,
        subject_kind: 'recommendation',
        subject_id: 'recommendation-1',
        capability: 'recommendation',
      })
    )
  );
});

test('a failed latest manifest fetch does not fall back to a cached report target', async () => {
  const fetchCommunityNodeManifest = vi.fn().mockResolvedValue({
    status: 'absent',
    manifest: null,
  });
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({
      entries: [indexEntry('post-a', 'report target requires a fresh manifest')],
    }),
    fetchCommunityNodeManifest,
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  runSearch();
  expect(await screen.findByText('report target requires a fresh manifest')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Report' }));

  const dialog = await screen.findByRole('dialog', { name: 'Report content' });
  await waitFor(() => expect(fetchCommunityNodeManifest).toHaveBeenCalledWith(NODE_A));
  expect(
    await within(dialog).findByText(
      'Could not refresh report targets. No default destination will be used.'
    )
  ).toBeInTheDocument();
  expect(within(dialog).queryByRole('button', { name: 'Send report' })).not.toBeInTheDocument();
});

test.each([
  [new InvokeError('AUTH_REQUIRED', 'server auth error', 401), '選択した Community Node への認証が必要です。'],
  [new InvokeError('CONSENT_REQUIRED', 'server consent error', 403), '選択した Community Node の必須同意が必要です。'],
  [new InvokeError('INDEX_QUERY_NOT_CONFIGURED', 'server config error'), '選択した Community Node ではコミュニティ索引が設定されていません。'],
  [new InvokeError('INDEX_QUERY_NOT_ACTIVATED', 'server activation error'), '選択した Community Node のコミュニティ索引は一時的に利用できません。'],
  [new InvokeError('RATE_LIMITED', 'server rate error', 429, 12), '要求が多すぎます。12秒後にもう一度お試しください。'],
])('known query error is localized in Japanese: %s', async (cause, expected) => {
  await i18n.changeLanguage('ja');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockRejectedValue(cause),
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api, { locale: 'ja' })} />);

  fireEvent.change(screen.getByLabelText('検索語'), { target: { value: '検索' } });
  fireEvent.click(screen.getByRole('button', { name: '実行' }));

  expect(await screen.findByText(expected)).toBeInTheDocument();
});

// #698: 選択値が適格一覧から外れている間は古いノードへ要求を送らない。
test('a selected node that is no longer eligible does not receive queries until it is eligible again', async () => {
  const searchCommunityNodeIndex = vi.fn().mockResolvedValue({ entries: [] });
  const api = { searchCommunityNodeIndex } as unknown as DesktopApi;
  const { rerender } = render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { eligibleNodeBaseUrls: [NODE_B], selectedNodeBaseUrl: NODE_A })}
    />
  );

  // 適格一覧 [B] と古い選択値 A が同時に渡っても、A へ検索語を送らない。
  const runButton = screen.queryByRole('button', { name: 'Run' });
  if (runButton) fireEvent.click(runButton);
  await new Promise((done) => setTimeout(done, 0));
  expect(searchCommunityNodeIndex).not.toHaveBeenCalled();

  // 再調整で選択が適格ノードになれば送れる。
  rerender(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { eligibleNodeBaseUrls: [NODE_B], selectedNodeBaseUrl: NODE_B })}
    />
  );
  runSearch('hello');
  await waitFor(() => expect(searchCommunityNodeIndex).toHaveBeenCalledTimes(1));
  expect(searchCommunityNodeIndex).toHaveBeenCalledWith(
    expect.objectContaining({ base_url: NODE_B, query: 'hello' })
  );
});
