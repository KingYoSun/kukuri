import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityNodePanel } from './CommunityNodePanel';
import { createCommunityNodePanelFixture } from './fixtures';

test('loads, enables, and clears reversible node-local distance opt-out', async () => {
  const getState = vi.fn(async () => ({
    pubkey: 'viewer',
    opted_out: false,
    opted_out_at: null,
    min_proximity: 0.25,
  }));
  const setState = vi.fn(async () => ({
    pubkey: 'viewer',
    opted_out: true,
    opted_out_at: '2026-08-14T00:00:00Z',
    min_proximity: 0.25,
  }));
  const clearState = vi.fn(async () => ({
    pubkey: 'viewer',
    opted_out: false,
    opted_out_at: null,
    min_proximity: 0.25,
  }));

  render(
    <CommunityNodePanel
      view={createCommunityNodePanelFixture()}
      saveDisabled={false}
      resetDisabled={false}
      clearDisabled={false}
      onAddNode={() => undefined}
      onNodeBaseUrlChange={() => undefined}
      onNodeAutoApproveChange={() => undefined}
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
      onGetRelationOptout={getState}
      onSetRelationOptout={setState}
      onClearRelationOptout={clearState}
    />
  );

  expect(screen.getAllByText(/not a privacy feature|プライバシー機能/).length).toBeGreaterThan(0);
  await userEvent.click(screen.getAllByRole('button', { name: /Load setting|設定を読み込む/ })[0]);
  expect(await screen.findByText(/0.25/)).toBeInTheDocument();

  await userEvent.click(screen.getAllByRole('button', { name: /Enable|有効にする/ })[0]);
  expect(await screen.findByText(/Enabled|有効です/)).toBeInTheDocument();

  await userEvent.click(screen.getAllByRole('button', { name: /Disable|解除する/ })[0]);
  expect(clearState).toHaveBeenCalledWith('https://api.kukuri.app');
});
