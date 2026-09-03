import { type ComponentProps } from 'react';
import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { AuthorSocialView, CommunityNodeManifest, DesktopApi, PostView } from '@/lib/api';
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

function knownAuthor(authorPubkey: string): AuthorSocialView {
  return {
    author_pubkey: authorPubkey,
    name: 'alice',
    display_name: 'Alice',
    about: null,
    picture_asset: {
      hash: 'avatar-hash',
      mime: 'image/png',
      bytes: 42,
      role: 'profile_avatar',
    },
    updated_at: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    friend_of_friend_via_pubkeys: [],
    provenance: null,
    muted: false,
    blocking: false,
    blocked_by: false,
  };
}

function resolvedPost(objectId: string, content = 'canonical content'): PostView {
  return {
    object_id: objectId,
    envelope_id: `envelope-${objectId}`,
    author_pubkey: `author-${objectId}`,
    author_name: 'alice',
    author_display_name: 'Alice',
    author_picture_asset: null,
    following: false,
    followed_by: false,
    mutual: false,
    friend_of_friend: false,
    provenance: null,
    withdrawal: null,
    content,
    content_status: 'Available',
    attachments: [],
    created_at: 42,
    reply_to: null,
    reply_preview: null,
    root_id: objectId,
    object_kind: 'post',
    published_topic_id: 'rust',
    origin_topic_id: 'rust',
    repost_of: null,
    repost_commentary: null,
    is_threadable: true,
    channel_id: null,
    audience_label: 'Public',
    reaction_summary: [],
    my_reactions: [],
  };
}

function resolvedIndexEntry(objectId: string, content = 'canonical content') {
  return {
    key: `public_topic:rust:${objectId}`,
    post: resolvedPost(objectId, content),
    capabilities: {
      open_thread: true,
      reply: true,
      repost: true,
      quote_repost: true,
      react: true,
      copy_link: true,
      bookmark: true,
      withdraw: false,
    },
  };
}

function readOnlyResolvedIndexEntry(objectId: string, content: string) {
  const resolved = resolvedIndexEntry(objectId, content);
  return {
    ...resolved,
    post: resolved.post
      ? {
          ...resolved.post,
          author_name: null,
          author_display_name: null,
        }
      : null,
    capabilities: {
      open_thread: false,
      reply: false,
      repost: false,
      quote_repost: false,
      react: false,
      copy_link: false,
      bookmark: false,
      withdraw: false,
    },
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
  const indexTextByObjectId = new Map<string, string>();
  const captureIndexMethod = (
    method: DesktopApi['searchCommunityNodeIndex'] | undefined
  ): DesktopApi['searchCommunityNodeIndex'] | undefined =>
    method
      ? async (request) => {
          const response = await method(request);
          for (const entry of response.entries) {
            indexTextByObjectId.set(entry.object_id, entry.text);
          }
          return response;
        }
      : undefined;
  const defaultResolve: DesktopApi['resolveCommunityIndexPosts'] = async (entries) => ({
    entries: entries.map((entry) =>
      readOnlyResolvedIndexEntry(
        entry.object_id,
        indexTextByObjectId.get(entry.object_id) ?? 'canonical content'
      )
    ),
  });
  const effectiveApi = {
    ...api,
    searchCommunityNodeIndex: captureIndexMethod(api.searchCommunityNodeIndex),
    discoverCommunityNodeIndex: captureIndexMethod(api.discoverCommunityNodeIndex),
    recommendCommunityNodeIndex: captureIndexMethod(api.recommendCommunityNodeIndex),
    resolveCommunityIndexPosts: api.resolveCommunityIndexPosts ?? vi.fn(defaultResolve),
  } as DesktopApi;
  return {
    api: effectiveApi,
    mode: 'topic',
    activeTopic: 'rust',
    activeTimelineScope: { kind: 'public' },
    eligibleNodeBaseUrls: [NODE_A, NODE_B],
    selectedNodeBaseUrl: NODE_A,
    onOpenCommunityNodeSettings: vi.fn(),
    onOpenAuthor: vi.fn(),
    onOpenThread: vi.fn(),
    onOpenThreadInTopic: vi.fn(),
    onReply: vi.fn(),
    onRepost: vi.fn(),
    onQuoteRepost: vi.fn(),
    onToggleReaction: vi.fn(),
    onBookmarkCustomReaction: vi.fn(),
    onReactionPickerOpen: vi.fn(),
    showBookmarkAction: true,
    bookmarkedPostIds: new Set<string>(),
    onToggleBookmark: vi.fn(),
    onWithdraw: vi.fn(),
    onActivateReference: vi.fn(),
    onCopyPostLink: vi.fn(),
    ...overrides,
  };
}

function runSearch(query = 'hello') {
  fireEvent.change(screen.getByLabelText('Search query'), { target: { value: query } });
  fireEvent.click(screen.getByRole('button', { name: 'Show results' }));
}

test('healthy query node selection stays automatic and out of the primary surface', () => {
  const api = {} as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  expect(screen.queryByLabelText('Search provider')).not.toBeInTheDocument();
  expect(screen.getByLabelText('Search query')).toBeInTheDocument();
});

test('topic search sends the active public scope and renders results with the shared post card', async () => {
  const user = userEvent.setup();
  const onOpenAuthor = vi.fn();
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
  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        knownAuthorsByPubkey: { 'author-1': knownAuthor('author-1') },
        mediaObjectUrls: { 'avatar-hash': 'blob:avatar-hash' },
        onOpenAuthor,
      })}
    />
  );

  runSearch();

  await waitFor(() => expect(searchCommunityNodeIndex).toHaveBeenCalledTimes(1));
  expect(searchCommunityNodeIndex).toHaveBeenCalledWith(
    expect.objectContaining({
      scope_kind: 'public_topic',
      scope_id: 'rust',
      query: 'hello',
    })
  );
  const result = await screen.findByText('derived-tag');
  expect(result.closest('article')).toHaveClass('post-card');
  expect(screen.getByText('Alice')).toBeInTheDocument();
  expect(screen.getByTestId('post-1-author-avatar').querySelector('img')).toHaveAttribute(
    'src',
    'blob:avatar-hash'
  );
  expect(screen.queryByText(/Search preview; may include derived tags/)).not.toBeInTheDocument();
  expect(screen.queryByText('rust')).not.toBeInTheDocument();
  expect(screen.queryByText('public_topic')).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Repost' })).not.toBeInTheDocument();
  await user.click(screen.getByRole('button', { name: 'Alice' }));
  expect(onOpenAuthor).toHaveBeenCalledWith('author-1');
});

test('Explore results expose the same post actions as the timeline', async () => {
  const user = userEvent.setup();
  const onReply = vi.fn();
  const onRepost = vi.fn();
  const onToggleBookmark = vi.fn();
  const searchCommunityNodeIndex = vi.fn().mockResolvedValue({
    entries: [indexEntry('explore-actions', 'actionable result')],
  });
  const resolveCommunityIndexPosts = vi.fn().mockResolvedValue({
    entries: [resolvedIndexEntry('explore-actions')],
  });
  const api = { searchCommunityNodeIndex, resolveCommunityIndexPosts } as unknown as DesktopApi;
  const interactiveActions = {
    onOpenThread: vi.fn(),
    onOpenThreadInTopic: vi.fn(),
    onReply,
    onRepost,
    onQuoteRepost: vi.fn(),
    onToggleReaction: vi.fn(),
    showBookmarkAction: true,
    onToggleBookmark,
    onCopyPostLink: vi.fn(),
  };
  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { mode: 'explore' })}
      {...interactiveActions}
    />
  );

  runSearch();

  const result = await screen.findByText('canonical content');
  const card = result.closest('article');
  if (!(card instanceof HTMLElement)) throw new Error('Explore result card not found');

  await waitFor(() => expect(resolveCommunityIndexPosts).toHaveBeenCalledTimes(1));

  expect(within(card).getByRole('button', { name: 'React' })).toBeEnabled();
  expect(within(card).getByRole('button', { name: 'Repost' })).toBeInTheDocument();
  expect(within(card).getByRole('button', { name: 'Reply' })).toBeInTheDocument();
  expect(within(card).getByRole('button', { name: 'Copy link' })).toBeInTheDocument();
  expect(within(card).getByRole('button', { name: 'Bookmark' })).toBeInTheDocument();
  expect(within(card).getByRole('button', { name: 'Report' })).toBeInTheDocument();

  await user.click(within(card).getByRole('button', { name: 'Reply' }));
  expect(onReply).toHaveBeenCalledWith(
    expect.objectContaining({
      object_id: 'explore-actions',
      published_topic_id: 'rust',
      is_threadable: true,
      content: 'canonical content',
    })
  );

  await user.click(within(card).getByRole('button', { name: 'Bookmark' }));
  expect(onToggleBookmark).toHaveBeenCalledWith(
    expect.objectContaining({ object_id: 'explore-actions', published_topic_id: 'rust' })
  );

  await user.click(within(card).getByRole('button', { name: 'Repost' }));
  await user.click(screen.getAllByRole('button', { name: 'Repost' })[1]);
  expect(onRepost).toHaveBeenCalledWith(
    expect.objectContaining({
      object_id: 'explore-actions',
      published_topic_id: 'rust',
      content: 'canonical content',
    })
  );
});

test('reaction results are refreshed into the Community Index card', async () => {
  const user = userEvent.setup();
  const entry = indexEntry('reaction-result', 'reaction preview');
  const first = resolvedIndexEntry(entry.object_id);
  const initialPost = {
    ...first.post,
    reaction_summary: [
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
        count: 1,
      },
    ],
  };
  const refreshedPost = {
    ...initialPost,
    reaction_summary: [{ ...initialPost.reaction_summary[0], count: 2 }],
    my_reactions: [
      {
        reaction_key_kind: 'emoji',
        normalized_reaction_key: 'emoji:👍',
        emoji: '👍',
        custom_asset: null,
      },
    ],
  };
  const resolveCommunityIndexPosts = vi
    .fn()
    .mockResolvedValueOnce({ entries: [{ ...first, post: initialPost }] })
    .mockResolvedValueOnce({ entries: [{ ...first, post: refreshedPost }] });
  const onToggleReaction = vi.fn().mockResolvedValue(undefined);
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts,
  } as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        onToggleReaction,
        knownAuthorsByPubkey: { [entry.author_pubkey]: knownAuthor(entry.author_pubkey) },
      })}
    />
  );
  runSearch();

  const firstChip = await screen.findByRole('button', { name: /👍\s*1/ });
  await user.click(firstChip);

  expect(onToggleReaction).toHaveBeenCalledWith(
    expect.objectContaining({ object_id: entry.object_id, content: 'canonical content' }),
    { kind: 'emoji', emoji: '👍' }
  );
  await waitFor(() => expect(resolveCommunityIndexPosts).toHaveBeenCalledTimes(2));
  expect(await screen.findByRole('button', { name: /👍\s*2/ })).toBeInTheDocument();
});

test('unresolved results stay fail-closed and expose only reporting and identifier actions', async () => {
  const entry = indexEntry('unresolved', 'read-only result');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({
      entries: [
        {
          key: `public_topic:rust:${entry.object_id}`,
          post: null,
          capabilities: {
            open_thread: false,
            reply: false,
            repost: false,
            quote_repost: false,
            react: false,
            copy_link: false,
            bookmark: false,
            withdraw: false,
          },
        },
      ],
    }),
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  await waitFor(() => expect(api.resolveCommunityIndexPosts).toHaveBeenCalledTimes(1));
  expect(screen.queryByText(entry.text)).not.toBeInTheDocument();
  const safePlaceholder = await screen.findByText(
    'Post content is unavailable because its safety labels could not be verified.'
  );
  const card = safePlaceholder.closest('article');
  if (!(card instanceof HTMLElement)) throw new Error('Explore result card not found');

  expect(within(card).queryByRole('button', { name: 'React' })).not.toBeInTheDocument();
  expect(within(card).queryByRole('button', { name: 'Repost' })).not.toBeInTheDocument();
  expect(within(card).queryByRole('button', { name: 'Reply' })).not.toBeInTheDocument();
  expect(within(card).queryByRole('button', { name: 'Copy link' })).not.toBeInTheDocument();
  expect(within(card).queryByRole('button', { name: 'Bookmark' })).not.toBeInTheDocument();
  expect(within(card).getByRole('button', { name: 'Report' })).toBeInTheDocument();
});

test('node-provided result text stays hidden while canonical resolution is pending', async () => {
  const entry = indexEntry('pending-resolution', 'untrusted pending result');
  const pending = deferred<{ entries: ReturnType<typeof resolvedIndexEntry>[] }>();
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockReturnValue(pending.promise),
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  await waitFor(() => expect(api.resolveCommunityIndexPosts).toHaveBeenCalledTimes(1));
  expect(screen.queryByText(entry.text)).not.toBeInTheDocument();
  expect(
    screen.getByText('Checking the post before showing its content…')
  ).toBeInTheDocument();
});

test('node-provided result text stays hidden when canonical resolution fails', async () => {
  const entry = indexEntry('failed-resolution', 'untrusted failed result');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockRejectedValue(new Error('resolver unavailable')),
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  await waitFor(() => expect(api.resolveCommunityIndexPosts).toHaveBeenCalledTimes(1));
  expect(screen.queryByText(entry.text)).not.toBeInTheDocument();
  expect(
    await screen.findByText(
      'Post content is unavailable because its safety labels could not be verified.'
    )
  ).toBeInTheDocument();
});

test('missing author profiles are resolved instead of being labeled unknown', async () => {
  const entry = indexEntry('remote-author', 'profile lookup result');
  const getAuthorSocialView = vi.fn().mockResolvedValue(knownAuthor(entry.author_pubkey));
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    getAuthorSocialView,
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  expect(await screen.findByText('Alice')).toBeInTheDocument();
  expect(getAuthorSocialView).toHaveBeenCalledTimes(1);
  expect(getAuthorSocialView).toHaveBeenCalledWith(entry.author_pubkey);
  expect(screen.queryByText('Unknown user')).not.toBeInTheDocument();
});

test('the local profile is used without a redundant author lookup', async () => {
  const entry = indexEntry('local-author', 'local profile result');
  const getAuthorSocialView = vi.fn();
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    getAuthorSocialView,
  } as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        localAuthorPubkey: entry.author_pubkey,
        localProfile: {
          pubkey: entry.author_pubkey,
          name: 'local-alice',
          display_name: 'Local Alice',
          about: null,
          picture_asset: null,
          updated_at: 42,
        },
      })}
    />
  );
  runSearch();

  expect(await screen.findByText('Local Alice')).toBeInTheDocument();
  expect(getAuthorSocialView).not.toHaveBeenCalled();
});

test('unknown author is used only after a fetched profile has no configured name', async () => {
  const entry = indexEntry('nameless-author', 'nameless profile result');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    getAuthorSocialView: vi.fn().mockResolvedValue({
      ...knownAuthor(entry.author_pubkey),
      name: null,
      display_name: null,
    }),
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  expect(await screen.findByText('Unknown user')).toBeInTheDocument();
  expect(screen.queryByText('User information unavailable')).not.toBeInTheDocument();
});

test('author lookup failures are distinct from fetched nameless profiles', async () => {
  const entry = indexEntry('failed-author', 'failed profile result');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
    resolveCommunityIndexPosts: vi.fn().mockResolvedValue({ entries: [] }),
    getAuthorSocialView: vi.fn().mockRejectedValue(new Error('offline')),
  } as unknown as DesktopApi;

  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);
  runSearch();

  expect(await screen.findByText('User information unavailable')).toBeInTheDocument();
  expect(screen.queryByText('Unknown user')).not.toBeInTheDocument();
});

test('index results hide identifiers and copy their complete values from context actions', async () => {
  const user = userEvent.setup();
  const clipboardWriteText = vi.fn().mockResolvedValue(undefined);
  Object.defineProperty(navigator, 'clipboard', {
    configurable: true,
    value: { writeText: clipboardWriteText },
  });
  const entry = indexEntry('post-context', 'context result');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({ entries: [entry] }),
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  runSearch();
  const resultText = await screen.findByText(entry.text);
  expect(screen.queryByText(new RegExp(entry.author_pubkey))).not.toBeInTheDocument();
  expect(screen.queryByText(new RegExp(entry.object_id))).not.toBeInTheDocument();

  const target = resultText.closest('article')?.querySelector('[data-testid="post-identifier-target"]');
  if (!(target instanceof HTMLElement)) throw new Error('index result target not found');
  fireEvent.contextMenu(target, { clientX: 40, clientY: 50 });
  await user.click(screen.getByRole('menuitem', { name: 'Copy user ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(entry.author_pubkey);

  target.focus();
  fireEvent.keyDown(target, { key: 'F10', shiftKey: true });
  await user.click(screen.getByRole('menuitem', { name: 'Copy post ID' }));
  expect(clipboardWriteText).toHaveBeenLastCalledWith(entry.object_id);
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

test('changing the Explore operation clears cards from the previous report context', async () => {
  const api = {
    searchCommunityNodeIndex: vi.fn().mockResolvedValue({
      entries: [indexEntry('search-result', 'search result from node A')],
    }),
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api, { mode: 'explore' })} />);

  runSearch();
  expect(await screen.findByText('search result from node A')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('tab', { name: 'Discover' }));

  expect(screen.queryByText('search result from node A')).not.toBeInTheDocument();
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
  fireEvent.click(screen.getByRole('button', { name: 'Show results' }));
  expect(await screen.findByText('recommended result')).toBeInTheDocument();
  fireEvent.click(screen.getByRole('button', { name: 'Report' }));

  const dialog = await screen.findByRole('dialog', { name: 'Report content' });
  await waitFor(() => expect(fetchCommunityNodeManifest).toHaveBeenCalledWith(NODE_A));
  // 著者情報の解決は非同期に settle する(Loading… → unavailable)ため、完了を待って検証する。
  expect(
    await within(dialog).findByText(/^Recommendation · User information unavailable$/)
  ).toBeInTheDocument();
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
  [new InvokeError('AUTH_REQUIRED', 'server auth error', 401), '選択したコミュニティノードへの認証が必要です。'],
  [new InvokeError('CONSENT_REQUIRED', 'server consent error', 403), '選択したコミュニティノードの必須同意が必要です。'],
  [new InvokeError('INDEX_QUERY_NOT_CONFIGURED', 'server config error'), '選択したコミュニティノードではコミュニティ索引が設定されていません。'],
  [new InvokeError('INDEX_QUERY_NOT_ACTIVATED', 'server activation error'), '選択したコミュニティノードのコミュニティ索引は一時的に利用できません。'],
  [new InvokeError('RATE_LIMITED', 'server rate error', 429, 12), '要求が多すぎます。12秒後にもう一度お試しください。'],
])('known query error is localized in Japanese: %s', async (cause, expected) => {
  await i18n.changeLanguage('ja');
  const api = {
    searchCommunityNodeIndex: vi.fn().mockRejectedValue(cause),
  } as unknown as DesktopApi;
  render(<CommunityIndexWorkspace {...workspaceProps(api)} />);

  fireEvent.change(screen.getByLabelText('検索語'), { target: { value: '検索' } });
  fireEvent.click(screen.getByRole('button', { name: '結果を表示' }));

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
  const runButton = screen.queryByRole('button', { name: 'Show results' });
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

test('an unavailable explicit node reports the stopped state instead of offering a query form', () => {
  const api = { searchCommunityNodeIndex: vi.fn() } as unknown as DesktopApi;
  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { eligibleNodeBaseUrls: [NODE_B], selectedNodeBaseUrl: null })}
    />
  );

  expect(
    screen.getByText('The explicitly selected Community Node is unavailable. Queries are paused.')
  ).toBeInTheDocument();
  expect(screen.queryByRole('button', { name: 'Show results' })).not.toBeInTheDocument();
});
