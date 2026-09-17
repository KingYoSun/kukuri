import type { Meta, StoryObj } from '@storybook/react-vite';

import type { CommunityNodeObservationSharingStatus } from '@/lib/api';

import { CommunityNodeObservationSharingField } from './CommunityNodeObservationSharingField';

// #1061: ブロック・ミュートの提供（CN の任意同意文書）。提供中・停止中・再同意待ち・
// 削除要求中・未公開の全 state を並べる。
const policy: NonNullable<CommunityNodeObservationSharingStatus['policy']> = {
  policy_slug: 'trust_observation_sharing',
  policy_version: 1,
  title: 'ブロック・ミュート観測の提供',
  body_markdown:
    '## 提供する情報\n\n- あなたの公開鍵と、ブロック・ミュートした相手の公開鍵\n- 操作の種類と状態、時刻、署名\n\n## 保持期間\n\n有効な記録は 180 日、解除された記録は 30 日で削除します。',
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
};

function status(
  overrides: Partial<CommunityNodeObservationSharingStatus> = {}
): CommunityNodeObservationSharingStatus {
  return {
    base_url: 'https://node.example',
    offered: true,
    policy,
    enabled: false,
    needs_reconsent: false,
    revocation_pending: false,
    pending_count: 0,
    ...overrides,
  };
}

function handlers(value: CommunityNodeObservationSharingStatus) {
  return {
    getObservationSharing: async () => value,
    enableObservationSharing: async () => status({ enabled: true }),
    disableObservationSharing: async () => status(),
  };
}

const meta = {
  title: 'Settings/CommunityNodeObservationSharingField',
  component: CommunityNodeObservationSharingField,
  args: {
    nodeId: 'node-1',
    baseUrl: 'https://node.example',
    language: 'ja',
    ...handlers(status()),
  },
} satisfies Meta<typeof CommunityNodeObservationSharingField>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Disabled: Story = {};

export const Enabled: Story = {
  args: { ...handlers(status({ enabled: true, pending_count: 2 })) },
};

export const NeedsReconsent: Story = {
  args: { ...handlers(status({ needs_reconsent: true })) },
};

export const RevocationPending: Story = {
  args: { ...handlers(status({ revocation_pending: true })) },
};

/// 任意文書を公開していないノードでは何も描画しない。
export const NotOffered: Story = {
  args: { ...handlers(status({ offered: false, policy: null })) },
};
