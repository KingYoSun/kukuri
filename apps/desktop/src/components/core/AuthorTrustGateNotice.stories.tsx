import type { Meta, StoryObj } from '@storybook/react-vite';

import type { AuthorTrustGateView } from '@/shell/authorTrustGates';

import { AuthorTrustGateNotice } from './AuthorTrustGateNotice';

// #1061: 折りたたんだ投稿の代替表示。理由の種類・採用ノードの有無・引用元・作者導線の
// 全 state を並べる（observer と件数は出さない。ADR 0026 §8.3）。
const AUTHOR = 'b'.repeat(64);

function gate(overrides: Partial<AuthorTrustGateView> = {}): AuthorTrustGateView {
  return {
    authorPubkey: AUTHOR,
    nodeBaseUrl: 'https://node.example',
    reasons: ['related_users_block_or_mute'],
    fromRepostSource: false,
    ...overrides,
  };
}

const meta = {
  title: 'Core/AuthorTrustGateNotice',
  component: AuthorTrustGateNotice,
  args: {
    gate: gate(),
    onReveal: () => {},
    onOpenAuthor: () => {},
  },
} satisfies Meta<typeof AuthorTrustGateNotice>;

export default meta;
type Story = StoryObj<typeof meta>;

export const RelatedUsersBlockOrMute: Story = {};

export const RiskSignals: Story = {
  args: { gate: gate({ reasons: ['risk_signals'] }) },
};

export const BothReasons: Story = {
  args: { gate: gate({ reasons: ['risk_signals', 'related_users_block_or_mute'] }) },
};

/// 引用・repost の元投稿の作成者による折りたたみ。
export const FromRepostSource: Story = {
  args: { gate: gate({ fromRepostSource: true }) },
};

/// 理由もノードも示されない場合（応答が最小限のとき）。
export const WithoutReasonsOrNode: Story = {
  args: { gate: gate({ reasons: [], nodeBaseUrl: null }) },
};

/// 作者詳細への導線を持たない一覧（配線していない surface）。
export const WithoutOpenAuthor: Story = {
  args: { onOpenAuthor: undefined },
};
