import type { Meta, StoryObj } from '@storybook/react-vite';

import type { DesktopLogsView } from '@/lib/desktopLogs';
import { developerLogFixtureSnapshot } from '@/mocks/api/developerLogs';

import { DeveloperLogViewer } from './DeveloperLogViewer';
import { SettingsStoryFrame } from './SettingsStoryFrame';

function fixtureView(overrides: Partial<DesktopLogsView> = {}): DesktopLogsView {
  const snapshot = developerLogFixtureSnapshot();
  return {
    entries: snapshot.entries,
    oldestSeq: snapshot.oldest_seq,
    nextSeq: snapshot.next_seq,
    maxEntries: snapshot.max_entries,
    maxBytes: snapshot.max_bytes,
    droppedOlder: false,
    gapSinceLastRefresh: false,
    ...overrides,
  };
}

const readyView = fixtureView();

const meta = {
  title: 'Settings/DeveloperLogViewer',
  component: DeveloperLogViewer,
  render: (args) => (
    <SettingsStoryFrame width='wide'>
      <div className='p-6'>
        <DeveloperLogViewer {...args} />
      </div>
    </SettingsStoryFrame>
  ),
  args: {
    status: 'ready',
    view: readyView,
    errorMessage: null,
    onRefresh: () => {},
  },
} satisfies Meta<typeof DeveloperLogViewer>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Ready: Story = {};

export const Loading: Story = {
  args: { status: 'loading', view: null },
};

export const Empty: Story = {
  args: {
    view: { ...readyView, entries: [], oldestSeq: null, nextSeq: 1 },
  },
};

/// 上限到達後: 古い行が落ち、前回更新との間も失われた状態。
export const LimitReached: Story = {
  args: {
    view: {
      ...readyView,
      oldestSeq: 2_401,
      nextSeq: 4_401,
      droppedOlder: true,
      gapSinceLastRefresh: true,
    },
  },
};

export const ReadError: Story = {
  args: {
    status: 'error',
    view: null,
    errorMessage: 'desktop command `read_desktop_logs` requires Ready startup state; current state is Initializing',
  },
};

export const Narrow: Story = {
  render: (args) => (
    <SettingsStoryFrame width='narrow'>
      <div className='p-4'>
        <DeveloperLogViewer {...args} />
      </div>
    </SettingsStoryFrame>
  ),
};
