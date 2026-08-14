import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import type { CommunityNodeAdmissionRejectionCode } from '@/lib/api';

import { CommunityNodePanel } from './CommunityNodePanel';
import { createCommunityNodePanelFixture } from './fixtures';

function inviteAdmissionFixture() {
  const fixture = createCommunityNodePanelFixture();
  return {
    ...fixture,
    nodes: fixture.nodes.map((node, index) => {
      if (index === 0) {
        return {
          ...node,
          inviteCodeSaved: false,
          admissionRejectionCode: 'INVITE_REQUIRED' as const,
        };
      }
      return {
        ...node,
        admissionRejectionCode: 'BANNED' as const,
      };
    }),
  };
}

test('submits a masked invite code only for the selected node', async () => {
  const user = userEvent.setup();
  const onSubmitInviteCode = vi.fn(async () => {});

  render(
    <CommunityNodePanel
      view={inviteAdmissionFixture()}
      saveDisabled={false}
      resetDisabled={false}
      clearDisabled={false}
      onAddNode={() => {}}
      onNodeBaseUrlChange={() => {}}
      onNodeAutoApproveChange={() => {}}
      onRemoveNode={() => {}}
      onSaveNodes={() => {}}
      onReset={() => {}}
      onClearNodes={() => {}}
      onAuthenticate={() => {}}
      onSubmitInviteCode={onSubmitInviteCode}
      onFetchConsents={() => {}}
      onAcceptConsents={() => {}}
      onRefresh={() => {}}
      onClearToken={() => {}}
    />
  );

  const inviteInput = screen.getByLabelText(/Invite code|招待コード/);
  expect(inviteInput).toHaveAttribute('type', 'password');
  await user.type(inviteInput, 'join-code');
  await user.click(
    screen.getByRole('button', {
      name: /Save invite code and authenticate|招待コードを保存して認証/,
    })
  );

  expect(onSubmitInviteCode).toHaveBeenCalledWith('https://api.kukuri.app', 'join-code');
  expect(inviteInput).toHaveValue('');

  const bannedHeading = screen.getByText(/stopped providing support services|補助機能の提供を停止/);
  const bannedSection = bannedHeading.closest('section');
  expect(bannedSection).not.toBeNull();
  expect(within(bannedSection!).queryByLabelText(/Invite code|招待コード/)).not.toBeInTheDocument();
  expect(
    within(bannedSection!).getByText(/Automatic retries are stopped|自動再試行は行いません/)
  ).toBeInTheDocument();
});

test('provides Japanese reasons and next steps for every stable admission code', () => {
  const translate = i18n.getFixedT('ja', 'settings');
  const codes: CommunityNodeAdmissionRejectionCode[] = [
    'INVITE_REQUIRED',
    'INVITE_INVALID',
    'INVITE_EXPIRED',
    'INVITE_EXHAUSTED',
    'INVITE_REVOKED',
    'NOT_ALLOWLISTED',
    'BANNED',
  ];

  for (const code of codes) {
    const reason = translate(`communityNode.admission.reasons.${code}`);
    const nextStep = translate(`communityNode.admission.nextSteps.${code}`);
    expect(reason).not.toContain('communityNode.admission');
    expect(nextStep).not.toContain('communityNode.admission');
    expect(reason).toMatch(/[ぁ-んァ-ヶ一-龠]/);
    expect(nextStep).toMatch(/[ぁ-んァ-ヶ一-龠]/);
  }
});
