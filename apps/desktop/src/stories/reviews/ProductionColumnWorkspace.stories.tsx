import type { Meta, StoryObj } from '@storybook/react-vite';

import { ProductionColumnWorkspaceStory } from './ProductionColumnWorkspaceStory';

const meta = {
  title: 'Review/ProductionColumnWorkspace',
  component: ProductionColumnWorkspaceStory,
  parameters: {
    layout: 'fullscreen',
    reviewCanvas: true,
  },
} satisfies Meta<typeof ProductionColumnWorkspaceStory>;

export default meta;

type Story = StoryObj<typeof meta>;

// mobile paging story 用の viewport 定義。preview iframe 自体を 390×844 に絞り、
// production の `@media (max-width: 759px)`(mobile-column-workspace.css)を実際に発火させる。
const REVIEW_VIEWPORT_OPTIONS = {
  mobile390: {
    name: 'Mobile 390',
    styles: { width: '390px', height: '844px' },
    type: 'mobile' as const,
  },
};

export const ScopedDraftsAndComposer: Story = {};
export const ControlCenterOpen: Story = {
  args: {
    initialControlCenterOpen: true,
  },
};
export const InteractiveProductionShell: Story = {};
export const VariableSpanWideSurfaces: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 3,
  },
};
export const MetaverseFourSpan: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 4,
  },
};

/**
 * Notifications / Messages Column の構成 review。
 * kind 'notifications' と 'messages' の Column を Timeline と並置し、
 * seed 済みの通知 3 件 / 会話 2 件で空 state にならないことを確認する。
 */
export const NotificationsAndMessagesColumns: Story = {
  globals: { shellWidth: 'review1440' },
  args: {
    scenario: 'activity-surfaces',
  },
};

/**
 * Stream Column を 1 span に固定した review(menu 操作なし)。
 * `.shell-stream-layout` が 1 track(chat が本編の下)へ落ちることを確認する。
 */
export const StreamOneSpan: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    streamSpan: 1,
    metaverseSpan: 3,
  },
};

/**
 * Metaverse Column を desktop 幅で 1 span に固定した review(menu 操作なし)。
 * container 幅が 68rem を下回るため HUD が viewport 上の overlay 配置になることを確認する。
 */
export const MetaverseOneSpan: Story = {
  globals: { shellWidth: 'ultrawide1920' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 1,
  },
};

/**
 * Community Node healthy の対比 story。
 * Explore Column に `community-node-unavailable-notice` が出ないことを確認する。
 */
export const CommunityNodeHealthy: Story = {
  globals: { shellWidth: 'review1440' },
  args: {
    scenario: 'explore-status',
  },
};

/**
 * Community Node unavailable の review。
 * Control Center trigger の accessible name が「Community node needs attention」になり、
 * Explore Column に inline Notice(`community-node-unavailable-notice`)が同時に見える。
 */
export const CommunityNodeUnavailable: Story = {
  globals: { shellWidth: 'review1440' },
  args: {
    scenario: 'explore-status',
    communityNodeUnavailable: true,
  },
};

/**
 * narrow Desktop(760px 相当)で多 span Column を含む Canvas の review。
 * 横 overflow が `.shell-column-canvas` 内に閉じ、document が横 scroll しないことを確認する。
 */
export const NarrowDesktopColumns: Story = {
  globals: { shellWidth: 'compact760' },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 3,
  },
};

/**
 * mobile paging の review。globals.viewport で preview iframe 自体を 390px にし、
 * snap / page indicator / gesture owner の mobile 規則(759px media query)を発火させる。
 */
export const MobilePagingAndImmersiveLifecycle: Story = {
  globals: {
    shellWidth: 'mobile390',
    viewport: { value: 'mobile390', isRotated: false },
  },
  parameters: {
    viewport: { options: REVIEW_VIEWPORT_OPTIONS },
  },
  args: {
    scenario: 'wide-surfaces',
    metaverseSpan: 4,
  },
};

/**
 * Reduced motion の review(scoped-drafts + Control Center open)。確認観点:
 * - Control Center drawer の slide が即時に完了する(motion token が 0 になる)
 * - Column 切替(page indicator / Control Center Focus)の scroll が即時になる
 *   (decorator が documentElement へ設定する data-reduced-motion='reduce' を
 *   `prefersReducedMotion()` helper が拾い、auto-scroll / scrollIntoView の JS も抑制される)
 */
export const ReducedMotionProduction: Story = {
  globals: { motion: 'reduce' },
  args: {
    initialControlCenterOpen: true,
  },
};
