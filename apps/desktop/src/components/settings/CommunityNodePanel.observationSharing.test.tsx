import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import type { CommunityNodeObservationSharingStatus } from '@/lib/api';

import { CommunityNodePanel } from './CommunityNodePanel';
import { createCommunityNodePanelFixture } from './fixtures';

// #1061: ノードごとに、この端末のブロック・ミュートを評価へ提供するかを選ぶ。
// 提供は任意同意文書への同意で成立し、既存分は同意時に選んだ場合だけ送る。

function status(
  overrides: Partial<CommunityNodeObservationSharingStatus> = {}
): CommunityNodeObservationSharingStatus {
  return {
    base_url: 'https://node.example',
    offered: true,
    policy: {
      policy_slug: 'trust_observation_sharing',
      policy_version: 2,
      title: 'ブロック・ミュート観測の提供',
      body_markdown: '## 提供する情報\n\n公開鍵と種別を送ります。',
      required: false,
      effective_date: '2026-09-18',
      language: 'ja',
      policy_snapshot_revision: 'snapshot-1',
      authoritative_language: 'ja',
      reference_translation: false,
      translation_revision: null,
      translation_of_version: null,
      fallback: false,
      requested_language: null,
      material_change: false,
      requires_reconsent: false,
      is_current: true,
      publication_status: 'current',
      published_at: null,
      retired_at: null,
      previous_policy_version: null,
      previous_policy_snapshot_revision: null,
      next_policy_version: null,
      next_policy_snapshot_revision: null,
    },
    enabled: false,
    needs_reconsent: false,
    revocation_pending: false,
    pending_count: 0,
    ...overrides,
  };
}

function renderPanel(handlers: {
  get: () => Promise<CommunityNodeObservationSharingStatus>;
  enable?: (request: unknown) => Promise<CommunityNodeObservationSharingStatus>;
  disable?: (baseUrl: string) => Promise<CommunityNodeObservationSharingStatus>;
}) {
  const view = createCommunityNodePanelFixture();
  const node = view.nodes[0];
  render(
    <CommunityNodePanel
      view={view}
      saveDisabled={false}
      resetDisabled={false}
      clearDisabled={false}
      observationSharing={{
        getObservationSharing: handlers.get,
        enableObservationSharing: handlers.enable ?? (async () => status({ enabled: true })),
        disableObservationSharing: handlers.disable ?? (async () => status()),
      }}
      onAddNode={() => undefined}
      onNodeBaseUrlChange={() => undefined}
      onRemoveNode={() => undefined}
      onSaveNodes={() => undefined}
      onReset={() => undefined}
      onClearNodes={() => undefined}
      onAuthenticate={() => undefined}
      onSubmitInviteCode={async () => undefined}
      onFetchConsents={() => undefined}
      onAcceptConsents={() => undefined}
      onRefresh={() => undefined}
      onClearToken={() => undefined}
    />
  );
  return node;
}

test('accepting the optional document starts sharing and can include existing entries', async () => {
  const enable = vi.fn(async () => status({ enabled: true }));
  const node = renderPanel({ get: async () => status(), enable });

  const section = await screen.findByTestId(`community-node-observation-sharing-${node.id}`);
  expect(within(section).getByText(/Not sharing|提供していません/)).toBeInTheDocument();
  await userEvent.click(
    screen.getByTestId(`community-node-observation-sharing-toggle-${node.id}`)
  );

  // 同意ダイアログは文書の本文と、既存分を送るかの選択（既定はオフ）を出す。
  const dialog = await screen.findByRole('dialog');
  expect(within(dialog).getByText(/公開鍵と種別を送ります/)).toBeInTheDocument();
  const includeExisting = screen.getByTestId(
    `community-node-observation-sharing-existing-${node.id}`
  );
  expect(includeExisting).not.toBeChecked();
  await userEvent.click(includeExisting);
  await userEvent.click(
    screen.getByTestId(`community-node-observation-sharing-accept-${node.id}`)
  );

  await waitFor(() => expect(enable).toHaveBeenCalledTimes(1));
  expect(enable).toHaveBeenCalledWith({
    base_url: node.baseUrl,
    policy_version: 2,
    policy_snapshot_revision: 'snapshot-1',
    language: expect.any(String),
    include_existing: true,
  });
  await waitFor(() =>
    expect(screen.getByText(/Sharing\.|提供中です/)).toBeInTheDocument()
  );
});

test('stopping sharing requests deletion without opening the document', async () => {
  const disable = vi.fn(async () => status({ revocation_pending: true }));
  const node = renderPanel({ get: async () => status({ enabled: true }), disable });

  const toggle = await screen.findByTestId(
    `community-node-observation-sharing-toggle-${node.id}`
  );
  await userEvent.click(toggle);

  await waitFor(() => expect(disable).toHaveBeenCalledWith(node.baseUrl));
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  await waitFor(() =>
    expect(
      screen.getByText(/deletion request to finish|記録の削除を要求中です/)
    ).toBeInTheDocument()
  );
});

test('a node without the optional document offers no sharing choice', async () => {
  const node = renderPanel({ get: async () => status({ offered: false, policy: null }) });

  await waitFor(() =>
    expect(
      screen.queryByTestId(`community-node-observation-sharing-${node.id}`)
    ).not.toBeInTheDocument()
  );
});

test('an updated document explains that sharing stopped until it is accepted again', async () => {
  const node = renderPanel({
    get: async () => status({ enabled: false, needs_reconsent: true }),
  });

  const section = await screen.findByTestId(`community-node-observation-sharing-${node.id}`);
  expect(
    within(section).getByText(/document changed|文書が更新されたため/)
  ).toBeInTheDocument();
});
