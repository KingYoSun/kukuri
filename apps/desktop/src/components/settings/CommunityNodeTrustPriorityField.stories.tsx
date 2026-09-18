import type { Meta, StoryObj } from '@storybook/react-vite';

import { CommunityNodeTrustPriorityField } from './CommunityNodeTrustPriorityField';

// #1061: 信頼値で投稿を折りたたむノードの採用順位。未選択・単独・複数・編集中の
// 全 state を並べる（未選択では照会も折りたたみも起きない）。
const NODES = ['https://api.kukuri.app', 'https://node.example', 'https://third.example'];

const meta = {
  title: 'Settings/CommunityNodeTrustPriorityField',
  component: CommunityNodeTrustPriorityField,
  args: {
    configuredBaseUrls: NODES,
    priority: [],
    onChange: () => {},
  },
} satisfies Meta<typeof CommunityNodeTrustPriorityField>;

export default meta;
type Story = StoryObj<typeof meta>;

/// 既定。選んでいないので折りたたみは起きない。
export const NoneSelected: Story = {};

export const SingleNode: Story = {
  args: { priority: [NODES[1]] },
};

/// 上位から順に、有効な評価を返したノードの判断を採る。
export const Ordered: Story = {
  args: { priority: [NODES[2], NODES[0]] },
};

/// ノード一覧を編集中は操作できない（保存してから並べ替える）。
export const Disabled: Story = {
  args: { priority: [NODES[0]], disabled: true },
};

/// ノードを 1 つも設定していない場合。
export const WithoutConfiguredNodes: Story = {
  args: { configuredBaseUrls: [] },
};
