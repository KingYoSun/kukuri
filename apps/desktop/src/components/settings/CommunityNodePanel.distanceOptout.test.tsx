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
      onWithdrawConsents={() => {}}
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

// #705: 適格でないノード(同意未承認・能力未提供など)では距離利用停止を操作できない。
test('disables distance opt-out actions for a node that is not eligible', async () => {
  const getState = vi.fn();
  const fixture = createCommunityNodePanelFixture();
  const view = {
    ...fixture,
    nodes: fixture.nodes.map((node) => ({ ...node, distanceOptoutEligible: false })),
  };

  render(
    <CommunityNodePanel
      view={view}
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
      onWithdrawConsents={() => {}}
      onRefresh={() => undefined}
      onClearToken={() => undefined}
      onGetRelationOptout={getState}
      onSetRelationOptout={vi.fn()}
      onClearRelationOptout={vi.fn()}
    />
  );

  expect(
    screen.getAllByText(/Distance opt-out is unavailable for this node|距離利用停止を扱えません/).length
  ).toBeGreaterThan(0);
  const loadButtons = screen.getAllByRole('button', { name: /Load setting|設定を読み込む/ });
  loadButtons.forEach((button) => expect(button).toBeDisabled());
  await userEvent.click(loadButtons[0]);
  expect(getState).not.toHaveBeenCalled();
});

test('explains distance opt-out failures with stable codes instead of raw server text', async () => {
  // #712: 未提供・失効・認証・同意を安定コードで判別し、英文メッセージを生表示しない。
  const { InvokeError } = await import('@/lib/api/invoke/error');
  const getState = vi.fn(async () => {
    throw new InvokeError(
      'RELATION_VISIBILITY_NOT_CONFIGURED',
      'this community node does not provide relation distance opt-out'
    );
  });

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
      onWithdrawConsents={() => {}}
      onRefresh={() => undefined}
      onClearToken={() => undefined}
      onGetRelationOptout={getState}
      onSetRelationOptout={async () => {
        throw new Error('unused');
      }}
      onClearRelationOptout={async () => {
        throw new Error('unused');
      }}
    />
  );

  await userEvent.click(screen.getAllByRole('button', { name: /Load setting|設定を読み込む/ })[0]);
  expect(
    await screen.findByText(
      /does not provide relation distance opt-out\.|距離利用停止を提供していません/
    )
  ).toBeInTheDocument();
  // サーバの生メッセージ(小文字の英文)をそのまま表示しない。
  expect(
    screen.queryByText('this community node does not provide relation distance opt-out')
  ).not.toBeInTheDocument();
});
