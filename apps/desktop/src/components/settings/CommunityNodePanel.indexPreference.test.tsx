import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityNodePanel } from './CommunityNodePanel';
import { communityNodePanelFixture } from './fixtures';

const baseProps = {
  view: communityNodePanelFixture,
  saveDisabled: false,
  resetDisabled: false,
  clearDisabled: false,
  onAddNode: () => {},
  onNodeBaseUrlChange: () => {},
  onNodeAutoApproveChange: () => {},
  onRemoveNode: () => {},
  onSaveNodes: () => {},
  onReset: () => {},
  onClearNodes: () => {},
  onAuthenticate: () => {},
  onFetchConsents: () => {},
  onAcceptConsents: () => {},
  onRefresh: () => {},
  onClearToken: () => {},
  onSubmitInviteCode: async () => {},
};

test('shows a manual unavailable preference and can return to automatic selection', async () => {
  const user = userEvent.setup();
  const onPreferenceChange = vi.fn();
  const unavailable = communityNodePanelFixture.nodes[1].baseUrl;
  render(
    <CommunityNodePanel
      {...baseProps}
      indexNodePreference={{ mode: 'manual', baseUrl: unavailable }}
      eligibleIndexNodeBaseUrls={[communityNodePanelFixture.nodes[0].baseUrl]}
      onIndexNodePreferenceChange={onPreferenceChange}
    />
  );

  const selector = screen.getByRole('combobox', { name: 'Community Index query node' });
  expect(selector).toHaveValue(unavailable);
  expect(screen.getByText('The explicitly selected node is currently unavailable.')).toBeInTheDocument();
  await user.selectOptions(selector, 'auto');
  expect(onPreferenceChange).toHaveBeenCalledWith({ mode: 'auto' });
});

test('selects a configured node explicitly', async () => {
  const user = userEvent.setup();
  const onPreferenceChange = vi.fn();
  render(
    <CommunityNodePanel
      {...baseProps}
      indexNodePreference={{ mode: 'auto' }}
      eligibleIndexNodeBaseUrls={communityNodePanelFixture.nodes.map((node) => node.baseUrl)}
      onIndexNodePreferenceChange={onPreferenceChange}
    />
  );
  await user.selectOptions(
    screen.getByRole('combobox', { name: 'Community Index query node' }),
    communityNodePanelFixture.nodes[1].baseUrl
  );
  expect(onPreferenceChange).toHaveBeenCalledWith({
    mode: 'manual',
    baseUrl: communityNodePanelFixture.nodes[1].baseUrl,
  });
});
