import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { expect, test, vi } from 'vitest';

import type { DesktopApi, PostView } from '@/lib/api';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';
import {
  INDEX_IMAGE_HASH,
  NODE_A,
  indexEntry,
  resolvedImageIndexEntry,
  runSearch,
  workspaceProps,
} from './CommunityIndexWorkspace.testSupport';

// #1055: 「見つける」の content advisory ゲート。desktop-runtime が issuer 照合済みの advisory
// だけを返すため、fixture は採用済みのものを置く。
const ADVISORY_ISSUER_NODE_ID = 'd'.repeat(64);

function advisoryIndexEntry(
  objectId: string,
  overrides: { label?: string; subjectKind?: 'post_id' | 'blob_cid' } = {}
) {
  return {
    ...indexEntry(objectId, 'indexed text'),
    content_advisories: [
      {
        issuer_node_id: ADVISORY_ISSUER_NODE_ID,
        subject_kind: overrides.subjectKind ?? ('blob_cid' as const),
        subject_id: overrides.subjectKind === 'post_id' ? objectId : INDEX_IMAGE_HASH,
        category: 'nsfw' as const,
        label: overrides.label ?? 'adult',
        confidence: 84,
        signal_id: 'signal-1',
        basis: 'classifier_score' as const,
      },
    ],
  };
}

function advisoryApi(objectId: string, overrides: Parameters<typeof advisoryIndexEntry>[1] = {}) {
  return {
    searchCommunityNodeIndex: vi
      .fn()
      .mockResolvedValue({ entries: [advisoryIndexEntry(objectId, overrides)] }),
    resolveCommunityIndexPosts: vi
      .fn()
      .mockResolvedValue({ entries: [resolvedImageIndexEntry(objectId)] }),
    fetchCommunityNodeManifest: vi.fn().mockResolvedValue({
      status: 'ok',
      manifest: {
        node_id: ADVISORY_ISSUER_NODE_ID,
        node_name: 'index-node.example',
        node_role: '',
        server_name: '',
        manifest_version: 'v1',
        // 申し立ての送信先になるには、node が trust signal の発行を宣言している必要がある
        // (`REPORT_CAPABILITY_REQUIREMENTS.trust_signal`)。
        capability_scope: {
          available_enabled: ['community_index', 'community_local_trust'],
          planned_enabled: [],
        },
        authority_scope: {
          applies_to: ['communities_indexed_by_this_node', 'trust_signals_issued_by_this_node'],
          does_not_apply_to: ['kukuri_network_as_a_whole'],
        },
        p2p_boundary: {
          identity_authority: false,
          profile_canonical_store: false,
          social_graph_canonical_store: false,
          content_truth_source: false,
          network_wide_authority: false,
        },
        abuse_contact: 'abuse@index-node.example',
        report_endpoint: `${NODE_A}/v1/report`,
        terms_url: '',
      },
    }),
    submitCommunityNodeReport: vi.fn(),
  };
}

// #1055 / AC-1 / AC-2: 表示設定 OFF では advisory 付き結果が self-label と同じ代替表示になり、
// 発行 node / 分類 / 確信度 / 根拠と申し立て導線を説明する。
test('an advisory-labeled result is gated and explains the issuing node', async () => {
  const api = advisoryApi('advisory-post') as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        mediaObjectUrls: { [INDEX_IMAGE_HASH]: 'blob:index-image' },
        adultContentEnabled: false,
      })}
    />
  );
  runSearch();

  expect(await screen.findByTestId('media-adult-gated-advisory-post')).toBeInTheDocument();
  expect(screen.queryByTestId('media-preview-advisory-post')).not.toBeInTheDocument();

  const advisory = await screen.findByTestId('post-advisory-gated-advisory-post');
  expect(advisory).toHaveTextContent('Community Node estimate');
  // 断定せず推定であることを示す。
  expect(advisory).toHaveTextContent('neither a label from the person who posted it');
  expect(advisory).toHaveTextContent('Possible sexual content');
  expect(advisory).toHaveTextContent('84 out of 100');
  expect(advisory).toHaveTextContent('Automated classifier score');
  await waitFor(() =>
    expect(screen.getByTestId('post-advisory-issuer-advisory-post')).toHaveTextContent(
      'index-node.example'
    )
  );
  expect(screen.getByTestId('post-advisory-issuer-advisory-post')).toHaveTextContent('dddddddd');
  expect(screen.getByTestId('post-advisory-appeal-advisory-post')).toBeInTheDocument();
});

// #1055 / AC-2: 申し立ては既存の通報 dialog を appeal mode で開き、対象 risk signal を伴う。
test('the advisory placeholder starts an appeal carrying the risk signal', async () => {
  const api = advisoryApi('appeal-post') as unknown as DesktopApi;
  (api.submitCommunityNodeReport as ReturnType<typeof vi.fn>).mockResolvedValue({
    report_id: 'report-1',
    accepted: true,
    disputed_risk_signal_id: 'signal-1',
    node_base_url: NODE_A,
  });

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { adultContentEnabled: false })}
    />
  );
  runSearch();

  fireEvent.click(await screen.findByTestId('post-advisory-appeal-appeal-post'));

  const submit = await screen.findByRole('button', { name: 'Submit appeal' });
  await waitFor(() => expect(submit).toBeEnabled());
  fireEvent.click(submit);

  await waitFor(() => expect(api.submitCommunityNodeReport).toHaveBeenCalledTimes(1));
  const request = (api.submitCommunityNodeReport as ReturnType<typeof vi.fn>).mock.calls[0][0];
  expect(request.appeal).toEqual({ risk_signal_id: 'signal-1' });
  // blob 対象の advisory は media として申し立てる(#707 と同じ subject 規則)。
  expect(request.subject_kind).toBe('media');
  expect(request.subject_id).toBe(INDEX_IMAGE_HASH);
});

// #1055 / AC-1: 表示設定 ON では advisory があっても通常表示に戻る。
test('enabling adult display renders an advisory-labeled result normally', async () => {
  const api = advisoryApi('advisory-on-post') as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        mediaObjectUrls: { [INDEX_IMAGE_HASH]: 'blob:index-image' },
        adultContentEnabled: true,
      })}
    />
  );
  runSearch();

  const preview = await screen.findByTestId('media-preview-advisory-on-post');
  expect(preview).toHaveAttribute('src', 'blob:index-image');
  expect(screen.queryByTestId('post-advisory-gated-advisory-on-post')).not.toBeInTheDocument();
});

// #1055: manifest を取得できなくても代替表示と説明は出す。発行元は host で示す。
test('an unavailable manifest still explains the advisory using the node host', async () => {
  const api = advisoryApi('advisory-no-manifest') as unknown as DesktopApi;
  (api.fetchCommunityNodeManifest as ReturnType<typeof vi.fn>).mockResolvedValue({
    status: 'absent',
    manifest: null,
  });

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { adultContentEnabled: false })}
    />
  );
  runSearch();

  const advisory = await screen.findByTestId('post-advisory-gated-advisory-no-manifest');
  expect(advisory).toHaveTextContent('index-a.example');
});

// #1055 / AC-3 / INVAR-3: advisory でゲート中の投稿はプリフェッチ対象へ公開しない。
// 表示設定 ON では公開する(ephemeral fetch へ進む)。
test('advisory-gated posts are withheld from media prefetch until display is enabled', async () => {
  const onResolvedPostsChange = vi.fn();
  const api = advisoryApi('advisory-prefetch') as unknown as DesktopApi;

  const { rerender } = render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { adultContentEnabled: false, onResolvedPostsChange })}
    />
  );
  runSearch();

  await waitFor(() => expect(api.resolveCommunityIndexPosts).toHaveBeenCalledTimes(1));
  await waitFor(() =>
    expect(screen.getByTestId('post-advisory-gated-advisory-prefetch')).toBeInTheDocument()
  );
  expect(
    (onResolvedPostsChange.mock.calls.at(-1)?.[0] as PostView[] | undefined) ?? []
  ).toEqual([]);

  rerender(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { adultContentEnabled: true, onResolvedPostsChange })}
    />
  );

  await waitFor(() => {
    const published = onResolvedPostsChange.mock.calls.at(-1)?.[0] as PostView[] | undefined;
    expect(published?.map((post) => post.object_id)).toEqual(['advisory-prefetch']);
  });
});

// #1055: 未知ラベルの advisory ではゲートせず、manifest も引かない。
test('an advisory with an unknown label does not gate the result', async () => {
  const api = advisoryApi('advisory-unknown-label', {
    label: 'experimental-future-label',
  }) as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        mediaObjectUrls: { [INDEX_IMAGE_HASH]: 'blob:index-image' },
        adultContentEnabled: false,
      })}
    />
  );
  runSearch();

  const preview = await screen.findByTestId('media-preview-advisory-unknown-label');
  expect(preview).toHaveAttribute('src', 'blob:index-image');
  expect(
    screen.queryByTestId('post-advisory-gated-advisory-unknown-label')
  ).not.toBeInTheDocument();
  expect(api.fetchCommunityNodeManifest).not.toHaveBeenCalled();
});

// #1107 / AC-6: 投稿への advisory で代替表示にした解決済み投稿は `onResolvedPostsChange` へ
// 公開しないため、その添付 blob をゲート集合として呼出元へ伝える。
test('a post-level advisory publishes the gated attachment hashes of the resolved post', async () => {
  const api = advisoryApi('post-advisory-post', { subjectKind: 'post_id' }) as unknown as DesktopApi;
  const onAdvisoryGatedMediaHashesChange = vi.fn();
  const onResolvedPostsChange = vi.fn();

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, { adultContentEnabled: false })}
      onAdvisoryGatedMediaHashesChange={onAdvisoryGatedMediaHashesChange}
      onResolvedPostsChange={onResolvedPostsChange}
    />
  );
  runSearch();

  expect(await screen.findByTestId('media-adult-gated-post-advisory-post')).toBeInTheDocument();
  await waitFor(() =>
    expect(onAdvisoryGatedMediaHashesChange).toHaveBeenLastCalledWith([INDEX_IMAGE_HASH])
  );
  const publishedPosts = onResolvedPostsChange.mock.calls.flatMap(
    ([posts]: [PostView[]]) => posts
  );
  expect(publishedPosts.map((post) => post.object_id)).not.toContain('post-advisory-post');
});

// #1107 / AC-6: advisory の無い結果でも、同じ blob が別の投稿でゲートされていればメディアだけを伏せる。
test('a plain result whose blob is gated elsewhere shows the shared-media placeholder', async () => {
  const api = {
    ...advisoryApi('plain-result-post'),
    searchCommunityNodeIndex: vi
      .fn()
      .mockResolvedValue({ entries: [indexEntry('plain-result-post', 'indexed text')] }),
  } as unknown as DesktopApi;

  render(
    <CommunityIndexWorkspace
      {...workspaceProps(api, {
        mediaObjectUrls: { [INDEX_IMAGE_HASH]: 'blob:index-image' },
        adultContentEnabled: false,
      })}
      gatedMediaHashes={[INDEX_IMAGE_HASH]}
    />
  );
  runSearch();

  const placeholder = await screen.findByTestId('media-adult-gated-plain-result-post');
  expect(placeholder).toHaveTextContent('treated as adult material on another post');
  expect(screen.queryByTestId('media-preview-plain-result-post')).not.toBeInTheDocument();
  expect(screen.queryByTestId('post-advisory-gated-plain-result-post')).not.toBeInTheDocument();
});
