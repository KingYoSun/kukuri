// #960: 検索成功 0 件の空状態が、検索先・照合対象・0 件の理由・次の行動を示すことを固定する。
// 空表示は query 成功時に限り、error / loading / node 切替では出さない(DESIGN 4.2、troubleshooting)。
import { type ComponentProps } from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { DesktopApi } from '@/lib/api';
import { InvokeError } from '@/lib/api/invoke/error';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

const NODE_A = 'https://index-a.example';
const NODE_B = 'https://index-b.example';
const PUBKEY = 'a1'.repeat(32);

// #975: 既定の索引状況は「自分の申請なし。対象付きで読んだ公開 topic は索引対象外」。
function emptyIndexStatus(request: { scope_kind: string | null; topic_id: string | null }) {
  return {
    requests: [],
    target: request.scope_kind
      ? { scope_kind: request.scope_kind, scope_id: request.topic_id, supported: false }
      : null,
  };
}

function emptyApi(overrides: Partial<DesktopApi> = {}): {
  api: DesktopApi;
  search: ReturnType<typeof vi.fn>;
  discover: ReturnType<typeof vi.fn>;
  indexStatus: ReturnType<typeof vi.fn>;
  mutations: ReturnType<typeof vi.fn>[];
} {
  const search = vi.fn().mockResolvedValue({ entries: [] });
  const discover = vi.fn().mockResolvedValue({ entries: [] });
  const indexStatus = vi.fn().mockImplementation(async (request) => emptyIndexStatus(request));
  const submitIndexing = vi.fn();
  const acceptConsents = vi.fn();
  const api = {
    searchCommunityNodeIndex: search,
    discoverCommunityNodeIndex: discover,
    recommendCommunityNodeIndex: discover,
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    readCommunityNodeIndexingStatus: indexStatus,
    submitCommunityNodeIndexingRequest: submitIndexing,
    acceptCommunityNodeConsents: acceptConsents,
    ...overrides,
  } as unknown as DesktopApi;
  return { api, search, discover, indexStatus, mutations: [submitIndexing, acceptConsents] };
}

const PUBLIC_STATUS_READ = {
  base_url: NODE_A,
  scope_kind: 'public_topic',
  topic_id: 'rust',
  channel_id: null,
  confirm_private_channel_secret_disclosure: false,
};
const LIST_ONLY_STATUS_READ = {
  base_url: NODE_A,
  scope_kind: null,
  topic_id: null,
  channel_id: null,
  confirm_private_channel_secret_disclosure: false,
};

function props(
  api: DesktopApi,
  overrides: Partial<ComponentProps<typeof CommunityIndexWorkspace>> = {}
): ComponentProps<typeof CommunityIndexWorkspace> {
  return {
    api,
    mode: 'topic',
    activeTopic: 'rust',
    activeTimelineScope: { kind: 'public' },
    eligibleNodeBaseUrls: [NODE_A, NODE_B],
    selectedNodeBaseUrl: NODE_A,
    onOpenCommunityNodeSettings: vi.fn(),
    onOpenConnectivitySettings: vi.fn(),
    onOpenTimeline: vi.fn(),
    onRequestIndexing: vi.fn(),
    onOpenAuthor: vi.fn(),
    ...overrides,
  };
}

function runSearch(query: string) {
  fireEvent.change(screen.getByLabelText('Search query'), { target: { value: query } });
  fireEvent.click(screen.getByRole('button', { name: 'Show results' }));
}

async function findEmptyState() {
  return screen.findByRole('status', { name: 'No matching posts found.' });
}

test('an empty topic search explains where it looked, what it matches, and the next actions', async () => {
  const { api, search, indexStatus, mutations } = emptyApi();
  const view = props(api);
  render(<CommunityIndexWorkspace {...view} />);

  runSearch('CliPeerA');
  const empty = await findEmptyState();
  const inside = within(empty);

  expect(inside.getByText(`Search provider: ${NODE_A}`)).toBeInTheDocument();
  expect(inside.getByText('Searched: posts indexed for this topic')).toBeInTheDocument();
  expect(inside.getByText(/Matches: the text of indexed posts/)).toBeInTheDocument();
  // #975: 公開 topic は対象付きで索引状況を読み、確定した「索引対象外・申請なし」を断定文で示す。
  expect(
    await inside.findByText('Indexing status: this topic is not indexed by this node and has no request.')
  ).toBeInTheDocument();
  expect(indexStatus).toHaveBeenCalledTimes(1);
  expect(indexStatus).toHaveBeenCalledWith(PUBLIC_STATUS_READ);
  expect(inside.getByText(/is not indexed by this node, so its posts are not indexed/)).toBeInTheDocument();
  expect(inside.queryByText(/may not be in this node’s index yet/)).not.toBeInTheDocument();
  expect(inside.queryByText(/may not be one this node indexes/)).not.toBeInTheDocument();

  fireEvent.click(inside.getByRole('button', { name: 'Request indexing' }));
  expect(view.onRequestIndexing).toHaveBeenCalledWith({ kind: 'public_topic', topicId: 'rust' });
  fireEvent.click(inside.getByRole('button', { name: 'Open timeline' }));
  expect(view.onOpenTimeline).toHaveBeenCalledTimes(1);
  fireEvent.click(inside.getByRole('button', { name: 'Open connection diagnostics' }));
  expect(view.onOpenConnectivitySettings).toHaveBeenCalledTimes(1);
  fireEvent.click(inside.getByRole('button', { name: 'Open Community Node Settings' }));
  expect(view.onOpenCommunityNodeSettings).toHaveBeenCalledTimes(1);

  fireEvent.click(inside.getByRole('button', { name: 'Search again' }));
  await waitFor(() => expect(search).toHaveBeenCalledTimes(2));
  expect(search).toHaveBeenLastCalledWith(expect.objectContaining({ query: 'CliPeerA', scope_id: 'rust' }));
  // 再検索の空結果ごとに索引状況を取り直す(保持しない)。
  await waitFor(() => expect(indexStatus).toHaveBeenCalledTimes(2));
  for (const mutation of mutations) expect(mutation).not.toHaveBeenCalled();
  expect(inside.queryByRole('button', { name: 'Open this user' })).not.toBeInTheDocument();
});

test('an empty private channel search offers indexing for the channel target', async () => {
  const { api } = emptyApi();
  const view = props(api, {
    activeTimelineScope: { kind: 'channel', channel_id: 'channel-1' },
    activeChannelLabel: 'Design room',
  });
  render(<CommunityIndexWorkspace {...view} />);

  runSearch('design');
  const empty = await findEmptyState();
  expect(within(empty).getByText('Searched: posts indexed for this private channel')).toBeInTheDocument();
  // INVAR-3: 非公開チャンネルでは自分の申請一覧だけを読み、所属証明(秘密値)は送らない。
  expect(
    await within(empty).findByText(/Whether this channel is indexed can be checked from/)
  ).toBeInTheDocument();
  expect(api.readCommunityNodeIndexingStatus).toHaveBeenCalledTimes(1);
  expect(api.readCommunityNodeIndexingStatus).toHaveBeenCalledWith(LIST_ONLY_STATUS_READ);
  expect(within(empty).getByText(/may not be one this node indexes/)).toBeInTheDocument();
  fireEvent.click(within(empty).getByRole('button', { name: 'Request indexing' }));
  expect(view.onRequestIndexing).toHaveBeenCalledWith({
    kind: 'private_channel',
    topicId: 'rust',
    channelId: 'channel-1',
    channelLabel: 'Design room',
  });
});

test('an empty Explore search points to the topic list instead of requesting indexing for one topic', async () => {
  const { api } = emptyApi();
  render(<CommunityIndexWorkspace {...props(api, { mode: 'explore' })} />);

  runSearch('CliPeerA');
  const empty = await findEmptyState();
  const inside = within(empty);
  expect(inside.getByText('Searched: all public topics indexed by this node')).toBeInTheDocument();
  expect(inside.getByText(/use “Request indexing” in the topic list/)).toBeInTheDocument();
  expect(inside.queryByRole('button', { name: 'Request indexing' })).not.toBeInTheDocument();
  // 横断検索は対象が一意でないため、自分の申請一覧だけを読む。
  expect(await inside.findByText('You have no indexing requests on this node.')).toBeInTheDocument();
  expect(api.readCommunityNodeIndexingStatus).toHaveBeenCalledWith(LIST_ONLY_STATUS_READ);
  expect(inside.getByText(/may not be in this node’s index yet/)).toBeInTheDocument();
  expect(inside.getByText(/may not be one this node indexes/)).toBeInTheDocument();
});

// #975 AC-3: 横断検索の空状態は、自分の申請を状態別に断定表示する。
test('an empty Explore search lists own indexing requests by status', async () => {
  const { api } = emptyApi({
    readCommunityNodeIndexingStatus: vi.fn().mockResolvedValue({
      requests: [
        { request_id: 'r1', scope_kind: 'public_topic', target_id: 'kukuri:topic:rust', status: 'approved', created_at: 1, decided_at: 2 },
        { request_id: 'r2', scope_kind: 'public_topic', target_id: 'kukuri:topic:golang', status: 'pending', created_at: 3, decided_at: null },
        { request_id: 'r3', scope_kind: 'private_channel', target_id: 'channel-9', status: 'rejected', created_at: 4, decided_at: 5 },
      ],
      target: null,
    }),
  } as Partial<DesktopApi>);
  render(<CommunityIndexWorkspace {...props(api, { mode: 'explore' })} />);

  runSearch('CliPeerA');
  const inside = within(await findEmptyState());
  expect(await inside.findByText('Approved requests: rust')).toBeInTheDocument();
  expect(inside.getByText('Pending requests: golang')).toBeInTheDocument();
  expect(inside.getByText('Rejected requests: channel-9')).toBeInTheDocument();
  expect(inside.queryByText('You have no indexing requests on this node.')).not.toBeInTheDocument();
});

// #975 AC-3 / AC-4: 確定した状態ごとに理由と申請 CTA が切り替わる。
test.each([
  {
    name: 'supported',
    response: { requests: [], target: { scope_kind: 'public_topic', scope_id: 'rust', supported: true } },
    statusText: 'Indexing status: this topic is indexed by this node.',
    reason: /may not be in this node’s index yet/,
    offersRequest: false,
  },
  {
    name: 'pending',
    response: {
      requests: [{ request_id: 'r1', scope_kind: 'public_topic', target_id: 'rust', status: 'pending', created_at: 1, decided_at: null }],
      target: { scope_kind: 'public_topic', scope_id: 'rust', supported: false },
    },
    statusText: 'Indexing status: the indexing request for this topic is pending review.',
    reason: /pending review, so nothing is indexed yet/,
    offersRequest: false,
  },
  {
    name: 'rejected',
    response: {
      requests: [{ request_id: 'r1', scope_kind: 'public_topic', target_id: 'rust', status: 'rejected', created_at: 1, decided_at: 2 }],
      target: { scope_kind: 'public_topic', scope_id: 'rust', supported: false },
    },
    statusText: 'Indexing status: the indexing request for this topic was rejected.',
    reason: /was rejected, so this node does not index it/,
    offersRequest: false,
  },
])('a $name topic shows a definite status', async ({ response, statusText, reason, offersRequest }) => {
  const { api } = emptyApi({
    readCommunityNodeIndexingStatus: vi.fn().mockResolvedValue(response),
  } as Partial<DesktopApi>);
  render(<CommunityIndexWorkspace {...props(api)} />);

  runSearch('CliPeerA');
  const inside = within(await findEmptyState());
  expect(await inside.findByText(statusText)).toBeInTheDocument();
  expect(inside.getByText(reason)).toBeInTheDocument();
  expect(inside.queryByText(/may not be one this node indexes/)).not.toBeInTheDocument();
  expect(inside.queryByRole('button', { name: 'Request indexing' }) !== null).toBe(offersRequest);
});

test('a failed status read keeps the possibilities and the request action instead of asserting a state', async () => {
  const { api } = emptyApi({
    readCommunityNodeIndexingStatus: vi.fn().mockRejectedValue(new InvokeError('INDEXING_REQUEST_NOT_ACTIVATED', 'gate')),
  } as Partial<DesktopApi>);
  render(<CommunityIndexWorkspace {...props(api)} />);

  runSearch('CliPeerA');
  const inside = within(await findEmptyState());
  expect(
    await inside.findByText(/The indexing status on this node could not be checked/)
  ).toBeInTheDocument();
  expect(inside.getByText(/may not be in this node’s index yet/)).toBeInTheDocument();
  expect(inside.getByText(/may not be one this node indexes/)).toBeInTheDocument();
  expect(inside.getByRole('button', { name: 'Request indexing' })).toBeInTheDocument();
});

test('a user ID query offers to open the user directly instead of matching post text', async () => {
  const { api } = emptyApi();
  const view = props(api, { mode: 'explore' });
  render(<CommunityIndexWorkspace {...view} />);

  runSearch(` ${PUBKEY.toUpperCase()} `);
  const empty = await findEmptyState();
  const inside = within(empty);
  expect(inside.getByText(/A user ID is not matched by search/)).toBeInTheDocument();
  expect(inside.queryByText(/Matches: the text of indexed posts/)).not.toBeInTheDocument();
  fireEvent.click(inside.getByRole('button', { name: 'Open this user' }));
  expect(view.onOpenAuthor).toHaveBeenCalledWith(PUBKEY);
});

test('an empty discovery result keeps the scope explanation without the text-matching rule', async () => {
  const { api, discover } = emptyApi();
  render(<CommunityIndexWorkspace {...props(api, { mode: 'explore' })} />);

  fireEvent.click(screen.getByRole('tab', { name: 'Discover' }));
  fireEvent.click(screen.getByRole('button', { name: 'Show results' }));
  const empty = await findEmptyState();
  const inside = within(empty);
  expect(inside.getByText('Searched: all public topics indexed by this node')).toBeInTheDocument();
  expect(inside.queryByText(/Matches: the text of indexed posts/)).not.toBeInTheDocument();
  expect(inside.getByText(/may not be in this node’s index yet/)).toBeInTheDocument();
  fireEvent.click(inside.getByRole('button', { name: 'Try again' }));
  await waitFor(() => expect(discover).toHaveBeenCalledTimes(2));
});

test('the empty state is shown only after a successful query and clears on error or node change', async () => {
  const search = vi
    .fn()
    .mockResolvedValueOnce({ entries: [] })
    .mockRejectedValueOnce(new InvokeError('RATE_LIMITED', 'slow down', 429));
  const { api } = emptyApi({ searchCommunityNodeIndex: search } as Partial<DesktopApi>);
  const view = props(api);
  const { rerender } = render(<CommunityIndexWorkspace {...view} />);

  expect(screen.queryByRole('status', { name: 'No matching posts found.' })).not.toBeInTheDocument();
  runSearch('first');
  await findEmptyState();

  runSearch('second');
  await screen.findByText('Too many requests. Please try again later.');
  expect(screen.queryByRole('status', { name: 'No matching posts found.' })).not.toBeInTheDocument();

  search.mockResolvedValueOnce({ entries: [] });
  rerender(<CommunityIndexWorkspace {...view} selectedNodeBaseUrl={NODE_B} />);
  await waitFor(() => expect(screen.queryByText('Too many requests. Please try again later.')).not.toBeInTheDocument());
  expect(screen.queryByRole('status', { name: 'No matching posts found.' })).not.toBeInTheDocument();
});

test('the Japanese empty state keeps the same explanation and actions', async () => {
  await i18n.changeLanguage('ja');
  try {
    const { api } = emptyApi();
    render(<CommunityIndexWorkspace {...props(api)} />);
    fireEvent.change(screen.getByLabelText('検索語'), { target: { value: 'CliPeerA' } });
    fireEvent.click(screen.getByRole('button', { name: '結果を表示' }));
    const empty = await screen.findByRole('status', { name: '該当する投稿は見つかりませんでした。' });
    const inside = within(empty);
    expect(inside.getByText('検索した範囲: このトピックで索引された投稿')).toBeInTheDocument();
    expect(inside.getByText(/ユーザー名やユーザーIDは、投稿本文に含まれていない限り一致しません/)).toBeInTheDocument();
    expect(inside.getByRole('button', { name: 'もう一度検索' })).toBeInTheDocument();
    expect(inside.getByRole('button', { name: 'タイムラインを見る' })).toBeInTheDocument();
    expect(inside.getByRole('button', { name: '索引登録を申請' })).toBeInTheDocument();
    expect(inside.getByRole('button', { name: '接続診断を開く' })).toBeInTheDocument();
  } finally {
    await i18n.changeLanguage('en');
  }
});
