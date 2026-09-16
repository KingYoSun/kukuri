import { fireEvent, screen } from '@testing-library/react';
import { type ComponentProps } from 'react';
import { vi } from 'vitest';

import type {
  AuthorSocialView,
  CommunityNodeManifest,
  DesktopApi,
  PostView,
} from '@/lib/api';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';

// 「見つける」Column の test が共有する seed / props / 操作。
// 本体の spec と #1055 の advisory spec が同じ fixture を使う。

export const NODE_A = 'https://index-a.example';
export const NODE_B = 'https://index-b.example';

export function manifestFor(baseUrl: string, nodeId: string): CommunityNodeManifest {
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

export const manifest = manifestFor(NODE_A, 'node-a');

export function indexEntry(objectId: string, text: string) {
  return {
    scope_kind: 'public_topic' as const,
    scope_id: 'rust',
    object_id: objectId,
    author_pubkey: `author-${objectId}`,
    text,
    created_at: 42,
  };
}

export function knownAuthor(authorPubkey: string): AuthorSocialView {
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

export function resolvedIndexEntry(objectId: string, content = 'canonical content') {
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

export function readOnlyResolvedIndexEntry(objectId: string, content: string) {
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

export function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((done) => {
    resolve = done;
  });
  return { promise, resolve };
}

export function workspaceProps(
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

export function runSearch(query = 'hello') {
  fireEvent.change(screen.getByLabelText('Search query'), { target: { value: query } });
  fireEvent.click(screen.getByRole('button', { name: 'Show results' }));
}

export const INDEX_IMAGE_HASH = 'b'.repeat(64);

export function resolvedImageIndexEntry(objectId: string, contentLabels: string[] = []) {
  const resolved = resolvedIndexEntry(objectId, 'canonical content with media');
  if (!resolved.post) throw new Error('resolved post fixture missing');
  return {
    ...resolved,
    post: {
      ...resolved.post,
      content_labels: contentLabels,
      attachments: [
        {
          hash: INDEX_IMAGE_HASH,
          mime: 'image/png',
          bytes: 2048,
          role: 'image_original',
          status: 'Available' as const,
        },
      ],
    },
  };
}
