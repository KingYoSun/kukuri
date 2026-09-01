import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityIndexWorkspace } from './CommunityIndexWorkspace';
import { createDesktopMockApi } from '@/mocks/desktopApiMock';

const NODE = 'https://api.kukuri.app';

function renderWorkspace(api = createDesktopMockApi()) {
  render(
    <CommunityIndexWorkspace
      api={api}
      mode='explore'
      activeTopic='kukuri:topic:general'
      activeTimelineScope={{ kind: 'public' }}
      eligibleNodeBaseUrls={[]}
      consentPendingNodeBaseUrls={[NODE]}
      selectedNodeBaseUrl={null}
      onOpenCommunityNodeSettings={() => {}}
      onOpenAuthor={() => {}}
    />
  );
  return api;
}

// #857 受入条件: Node 機能の利用直前にのみ、対象 Node の同意モーダルが表示される。
test('community index surfaces a consent prompt and per-node policy modal before first use', async () => {
  const user = userEvent.setup();
  const api = renderWorkspace();
  const fetchPolicies = vi.spyOn(api, 'fetchCommunityNodePolicies');
  const acceptConsents = vi.spyOn(api, 'acceptCommunityNodeConsents');

  expect(
    screen.getByText(
      'A community node requires your consent before it can be used. Review its terms and policies to connect.'
    )
  ).toBeInTheDocument();
  // モーダルは利用直前(ユーザー操作)まで開かない。
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(fetchPolicies).not.toHaveBeenCalled();

  await user.click(screen.getByRole('button', { name: `Review policies: ${NODE}` }));

  const dialog = await screen.findByRole('dialog');
  expect(fetchPolicies).toHaveBeenCalledWith(NODE);
  // 公開カタログ由来の文書本文と版が提示される。
  expect(
    await within(dialog).findByText('You must follow the community node terms of service.')
  ).toBeInTheDocument();
  expect(
    within(dialog).getByText('You must acknowledge the community node privacy policy.')
  ).toBeInTheDocument();

  await user.click(within(dialog).getByRole('button', { name: 'Accept' }));

  // 提示された文書と版をそのまま受諾し、表示言語を記録用に渡す。
  expect(acceptConsents).toHaveBeenCalledWith(
    NODE,
    [
      { policy_slug: 'terms_of_service', policy_version: 1 },
      { policy_slug: 'privacy_policy', policy_version: 1 },
    ],
    expect.any(String)
  );
  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
});

// #857 受入条件: 不同意でもモーダルを閉じるだけで、非 Node 機能はブロックされない。
test('declining the consent modal closes it without accepting', async () => {
  const user = userEvent.setup();
  const api = renderWorkspace();
  const acceptConsents = vi.spyOn(api, 'acceptCommunityNodeConsents');

  await user.click(screen.getByRole('button', { name: `Review policies: ${NODE}` }));
  const dialog = await screen.findByRole('dialog');
  await within(dialog).findByText('You must follow the community node terms of service.');

  await user.click(within(dialog).getByRole('button', { name: 'Not now' }));

  expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  expect(acceptConsents).not.toHaveBeenCalled();
});
