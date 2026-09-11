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

function emptyApi(overrides: Partial<DesktopApi> = {}): {
  api: DesktopApi;
  search: ReturnType<typeof vi.fn>;
  discover: ReturnType<typeof vi.fn>;
  mutations: ReturnType<typeof vi.fn>[];
} {
  const search = vi.fn().mockResolvedValue({ entries: [] });
  const discover = vi.fn().mockResolvedValue({ entries: [] });
  const submitIndexing = vi.fn();
  const acceptConsents = vi.fn();
  const api = {
    searchCommunityNodeIndex: search,
    discoverCommunityNodeIndex: discover,
    recommendCommunityNodeIndex: discover,
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    submitCommunityNodeIndexingRequest: submitIndexing,
    acceptCommunityNodeConsents: acceptConsents,
    ...overrides,
  } as unknown as DesktopApi;
  return { api, search, discover, mutations: [submitIndexing, acceptConsents] };
}

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
  const { api, search, mutations } = emptyApi();
  const view = props(api);
  render(<CommunityIndexWorkspace {...view} />);

  runSearch('CliPeerA');
  const empty = await findEmptyState();
  const inside = within(empty);

  expect(inside.getByText(`Search provider: ${NODE_A}`)).toBeInTheDocument();
  expect(inside.getByText('Searched: posts indexed for this topic')).toBeInTheDocument();
  expect(inside.getByText(/Matches: the text of indexed posts/)).toBeInTheDocument();
  expect(inside.getByText(/may not be in this node’s index yet/)).toBeInTheDocument();
  expect(inside.getByText(/may not be one this node indexes/)).toBeInTheDocument();

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
