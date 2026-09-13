import type { Meta, StoryObj } from '@storybook/react-vite';

import { BookmarksEmptyState, BookmarksListFrame } from './BookmarksEmptyState';

// #994: ブックマーク一覧の空状態と、その手前の loading / error 通知。Column 実幅(440px)で確認する。
const meta = {
  title: 'Core/BookmarksEmptyState',
  component: BookmarksEmptyState,
  decorators: [
    (Story) => (
      <div className='shell-phase1' style={{ width: 'min(440px, 100vw)', padding: 0 }}>
        <div className='shell-column-body'>
          <div className='shell-main-stack'>
            <Story />
          </div>
        </div>
      </div>
    ),
  ],
  args: {
    onShowTimeline: () => {},
  },
} satisfies Meta<typeof BookmarksEmptyState>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {};

export const WithoutTimelineAction: Story = { args: { onShowTimeline: undefined } };

export const Loading: Story = {
  render: () => (
    <BookmarksListFrame status='loading' error={null} onRetry={() => {}}>
      {() => null}
    </BookmarksListFrame>
  ),
};

export const LoadFailed: Story = {
  render: () => (
    <BookmarksListFrame status='error' error='Failed to load bookmarks.' onRetry={() => {}}>
      {() => null}
    </BookmarksListFrame>
  ),
};
