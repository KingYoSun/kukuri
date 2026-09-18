import type { Meta, StoryObj } from '@storybook/react-vite';

import { AuthorTrustDisplayExceptionField } from './AuthorTrustDisplayExceptionField';

// #1061: 作者詳細の「この作者を常に表示する」。未設定・設定済み・読めない・失敗の
// 全 state を並べる（端末内の設定で、ブロック・ミュートも CN の評価も変えない）。
const AUTHOR = 'b'.repeat(64);

const meta = {
  title: 'Core/AuthorTrustDisplayExceptionField',
  component: AuthorTrustDisplayExceptionField,
  args: {
    authorPubkey: AUTHOR,
    loadAlwaysVisible: async (): Promise<boolean> => false,
    setAlwaysVisible: async (_authorPubkey: string, alwaysVisible: boolean) => alwaysVisible,
  },
} satisfies Meta<typeof AuthorTrustDisplayExceptionField>;

export default meta;
type Story = StoryObj<typeof meta>;

export const NotExcepted: Story = {};

export const Excepted: Story = {
  args: { loadAlwaysVisible: async (): Promise<boolean> => true },
};

/// 設定を読めない場合は何も描画しない（折りたたみの判断には影響しない）。
export const Unavailable: Story = {
  args: {
    loadAlwaysVisible: async () => {
      throw new Error('unavailable');
    },
  },
};

/// 設定の保存に失敗した場合。
export const SaveFailed: Story = {
  args: {
    setAlwaysVisible: async () => {
      throw new Error('failed');
    },
  },
};
