import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

import { createDesktopMockApi } from '@/mocks/desktopApiMock';
import {
  getActiveColumn,
  openChannelManager,
  openControlCenter,
  renderAtHash,
  setViewportWidth,
} from './DesktopShellPage.testHelpers';

// Issue #966: 開発者モード OFF・未参加の通常画面から、プライベートチャンネルの存在と
// 作成・招待による参加・招待共有の入口へ到達できることを固定する。
// 案内を開くだけでは作成・参加・共有 API を呼ばず(INVAR-2)、閉じても topic / scope を変えない(INVAR-3)。

const DEMO_TOPIC_HASH = '#/timeline?topic=kukuri%3Atopic%3Ageneral';

beforeEach(() => {
  setViewportWidth(1024);
  window.history.replaceState(null, '', '/');
});

afterEach(() => {
  vi.useRealTimers();
});

function spiedApi() {
  const api = createDesktopMockApi();
  return {
    api,
    createSpy: vi.spyOn(api, 'createPrivateChannel'),
    exportSpy: vi.spyOn(api, 'exportChannelAccessToken'),
    importSpy: vi.spyOn(api, 'importChannelAccessToken'),
  };
}

test('Timeline Column header opens the create/join dialog with guidance and without channel API calls', async () => {
  const user = userEvent.setup();
  const { api, createSpy, exportSpy, importSpy } = spiedApi();
  renderAtHash(DEMO_TOPIC_HASH, api);

  const column = getActiveColumn('Timeline');
  const entry = within(column).getByRole('button', { name: 'Create or join a private channel' });
  expect(entry).toHaveTextContent('Private channel');
  await user.click(entry);

  const dialog = await screen.findByRole('dialog', { name: 'Create / Join Private Channel' });
  expect(within(dialog).getByText('general')).toBeInTheDocument();
  expect(
    within(dialog).getByText(/A private channel is a space inside this topic where only participants/)
  ).toBeInTheDocument();
  expect(
    within(dialog).getByText(/Invite and share links \(or tokens\) come from a channel participant/)
  ).toBeInTheDocument();
  expect(within(dialog).getByLabelText('Audience')).toHaveAccessibleDescription(
    /Invite only: only users who receive an invite link can join/
  );
  expect(createSpy).not.toHaveBeenCalled();
  expect(exportSpy).not.toHaveBeenCalled();
  expect(importSpy).not.toHaveBeenCalled();

  await user.keyboard('{Escape}');
  await waitFor(() => {
    expect(
      screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
    ).not.toBeInTheDocument();
  });
  expect(window.location.hash).toBe(DEMO_TOPIC_HASH);
  expect(getActiveColumn('Timeline')).toHaveTextContent('Public · general');
  expect(createSpy).not.toHaveBeenCalled();
});

test('channel scope Column exposes settings & sharing that opens Channel Settings for that channel', async () => {
  const user = userEvent.setup();
  const { api, exportSpy } = spiedApi();
  renderAtHash(DEMO_TOPIC_HASH, api);

  const createDialog = await openChannelManager(user);
  await user.type(within(createDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(createDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(window.location.hash).toBe(`${DEMO_TOPIC_HASH}&channel=channel-1`);
  });
  // 作成時の 1 回だけ(既存挙動)。
  expect(exportSpy).toHaveBeenCalledTimes(1);
  await user.click(within(createDialog).getByRole('button', { name: 'Close dialog' }));

  const column = getActiveColumn('Timeline');
  expect(column).toHaveTextContent('core · general');
  const entry = within(column).getByRole('button', {
    name: 'Open core channel settings and sharing',
  });
  expect(entry).toHaveTextContent('Settings & sharing');
  await user.click(entry);

  const settings = await screen.findByRole('dialog', { name: 'Channel Settings' });
  expect(within(settings).getByText('Channel name: core')).toBeInTheDocument();
  expect(exportSpy).toHaveBeenCalledTimes(1);
  await user.click(within(settings).getByRole('button', { name: 'Create share link' }));
  expect(await within(settings).findByText('Copy share link')).toBeInTheDocument();
  expect(exportSpy).toHaveBeenCalledTimes(2);
});

test('create/join dialog lists joined channels and hands off to their settings', async () => {
  const user = userEvent.setup();
  renderAtHash(DEMO_TOPIC_HASH);

  const createDialog = await openChannelManager(user);
  await user.type(within(createDialog).getByPlaceholderText('Channel name'), 'core');
  await user.click(within(createDialog).getByRole('button', { name: 'Create Channel' }));
  await waitFor(() => {
    expect(window.location.hash).toBe(`${DEMO_TOPIC_HASH}&channel=channel-1`);
  });
  expect(within(createDialog).getByText('Joined in this topic')).toBeInTheDocument();
  await user.click(within(createDialog).getByRole('button', { name: 'core settings and sharing' }));

  const settings = await screen.findByRole('dialog', { name: 'Channel Settings' });
  expect(within(settings).getByText('Channel name: core')).toBeInTheDocument();
  expect(
    screen.queryByRole('dialog', { name: 'Create / Join Private Channel' })
  ).not.toBeInTheDocument();
});

test('Control Center places explain the empty channel state and open the dialog for that topic', async () => {
  const user = userEvent.setup();
  const { api, createSpy } = spiedApi();
  renderAtHash(DEMO_TOPIC_HASH, api);

  const controlCenter = await openControlCenter(user);
  expect(within(controlCenter).getAllByText('No joined private channels.')).toHaveLength(3);
  const share = within(controlCenter).getByRole('button', { name: 'Share active channel' });
  expect(share).toBeDisabled();
  expect(share).toHaveAccessibleDescription(
    'Select a joined channel to create invite or share links.'
  );

  const devItem = within(controlCenter)
    .getByRole('button', { name: 'dev' })
    .closest('.topic-item');
  if (!(devItem instanceof HTMLElement)) throw new Error('expected dev topic item');
  await user.click(within(devItem).getByRole('button', { name: 'Create or join' }));

  const dialog = await screen.findByRole('dialog', { name: 'Create / Join Private Channel' });
  expect(within(dialog).getByText('dev')).toBeInTheDocument();
  expect(createSpy).not.toHaveBeenCalled();
});
