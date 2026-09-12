import type { Meta, StoryObj } from '@storybook/react-vite';
import { expect, mocked, userEvent, within } from 'storybook/test';

import { isTauriRuntime } from '@/lib/releaseReadiness';
import { invokeDesktop } from '@/lib/api/invoke/desktop';

import { createDesktopShellStore, DesktopShellStoreContext } from '@/shell/store';
import { appUpdateStore, INITIAL_UPDATE_STATE } from '@/shell/useAppUpdateStore';

import { ReleasePanel } from './ReleasePanel';
import { SettingsStoryFrame } from './SettingsStoryFrame';

const store = createDesktopShellStore();

const meta = {
  title: 'Settings/ReleasePanel',
  component: ReleasePanel,
  render: (args) => (
    <DesktopShellStoreContext.Provider value={store}>
      <SettingsStoryFrame width='wide'>
        <ReleasePanel {...args} />
      </SettingsStoryFrame>
    </DesktopShellStoreContext.Provider>
  ),
  args: {
    showDiagnostics: true,
  },
} satisfies Meta<typeof ReleasePanel>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Resources: Story = {};

function updateStatus(status: 'idle' | 'checking' | 'up_to_date' | 'available' | 'failed') {
  const previous = appUpdateStore.getState();
  appUpdateStore.setState({
    updateState: {
      ...INITIAL_UPDATE_STATE, status,
      availableVersion: status === 'available' ? '0.2.2-preview.1' : null,
      lastError: status === 'failed' ? 'network unavailable' : null,
      lastCheckedAt: ['up_to_date', 'available', 'failed'].includes(status)
        ? Date.UTC(2026, 8, 12, 3, 4, 0)
        : null,
    },
    pendingUpdate: status === 'available'
      ? { version: '0.2.2-preview.1', download: async () => {}, install: async () => {} }
      : null,
  });
  return () => appUpdateStore.setState(previous);
}

export const UpdateIdle: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => updateStatus('idle'),
};

export const UpdateChecking: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => updateStatus('checking'),
};

export const UpToDate: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => updateStatus('up_to_date'),
};

export const UpdateAvailable: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => updateStatus('available'),
};

export const UpdateCheckFailed: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => updateStatus('failed'),
};

function externalLinkState(pending: boolean) {
  mocked(isTauriRuntime).mockReturnValue(true);
  mocked(invokeDesktop).mockImplementation((command) => {
    if (command !== 'open_external_url') return Promise.resolve('available');
    return pending ? new Promise(() => {}) : Promise.reject(new Error('fixture'));
  });
  return () => {
    mocked(isTauriRuntime).mockRestore();
    mocked(invokeDesktop).mockRestore();
  };
}

export const ExternalLinkOpening: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => externalLinkState(true),
  play: async ({ canvasElement }) => {
    within(canvasElement).getAllByRole('link')[0].click();
  },
};

export const ExternalLinkFailed: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => externalLinkState(false),
  play: async ({ canvasElement }) => {
    within(canvasElement).getAllByRole('link')[0].click();
    await within(canvasElement).findByRole('alert');
  },
};

export const Installing: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => {
    const previous = appUpdateStore.getState();
    appUpdateStore.setState({
      updateState: { ...INITIAL_UPDATE_STATE, status: 'installing', availableVersion: '0.1.9' },
      pendingUpdate: null,
    });
    return () => appUpdateStore.setState(previous);
  },
};

export const ReadyToRestart: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => {
    const previous = appUpdateStore.getState();
    appUpdateStore.setState({
      updateState: { ...INITIAL_UPDATE_STATE, status: 'ready_to_restart', availableVersion: '0.1.9' },
      pendingUpdate: { version: '0.1.9', download: async () => {}, install: async () => {} },
      restartAndInstall: async () => {},
    });
    return () => appUpdateStore.setState(previous);
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole('button', { name: /^(Check|確認|检查)$/ })).toBeDisabled();
    await userEvent.click(canvas.getByRole('button', { name: /^(Later|あとで|稍后)$/ }));
    await expect(canvas.getByRole('button', { name: /^(Check|確認|检查)$/ })).toBeDisabled();
  },
};

export const MissingUpdateAsset: Story = {
  args: { showDiagnostics: false },
  beforeEach: () => {
    const previous = appUpdateStore.getState();
    appUpdateStore.setState({
      updateState: {
        ...INITIAL_UPDATE_STATE,
        status: 'failed',
        lastError: 'Download request failed with status: 404 Not Found',
      },
      pendingUpdate: null,
      checkForUpdate: async () => {},
    });
    return () => appUpdateStore.setState(previous);
  },
};

export const ReadyToRestartPrompt: Story = { ...ReadyToRestart, play: undefined };
