import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityNodePanel } from './CommunityNodePanel';
import { createCommunityNodePanelFixture } from './fixtures';

// #1056: node ごとに content advisory(成人向け表現の推定)を採用するかを切り替える。
// 切替は draft へ反映され、既存の「ノードを保存」で確定する。

function renderPanel(options: {
  contentAdvisoryEnabled?: boolean;
  onNodeContentAdvisoryChange?: (id: string, enabled: boolean) => void;
}) {
  const view = createCommunityNodePanelFixture();
  const node = { ...view.nodes[0], contentAdvisoryEnabled: options.contentAdvisoryEnabled };
  render(
    <CommunityNodePanel
      view={{ ...view, nodes: [node] }}
      saveDisabled={false}
      resetDisabled={false}
      clearDisabled={false}
      onAddNode={() => undefined}
      onNodeBaseUrlChange={() => undefined}
      onRemoveNode={() => undefined}
      onNodeContentAdvisoryChange={options.onNodeContentAdvisoryChange}
      onSaveNodes={() => undefined}
      onReset={() => undefined}
      onClearNodes={() => undefined}
      onAuthenticate={() => undefined}
      onSubmitInviteCode={async () => undefined}
      onFetchConsents={() => undefined}
      onAcceptConsents={() => undefined}
      onRefresh={() => undefined}
      onClearToken={() => undefined}
    />
  );
  return node;
}

test('shows the adoption toggle checked by default and explains what is sent', async () => {
  const onChange = vi.fn();
  const node = renderPanel({ onNodeContentAdvisoryChange: onChange });

  const section = screen.getByTestId(`community-node-advisory-adoption-${node.id}`);
  const toggle = within(section).getByRole('checkbox', {
    name: "Use this node's estimates of adult material",
  });
  expect(toggle).toBeChecked();
  expect(toggle).toHaveAccessibleDescription(/identifiers of their attachments are sent/);

  await userEvent.click(toggle);
  expect(onChange).toHaveBeenCalledWith(node.id, false);
});

test('explains that a disabled node is not asked', async () => {
  const onChange = vi.fn();
  const node = renderPanel({ contentAdvisoryEnabled: false, onNodeContentAdvisoryChange: onChange });

  const toggle = within(
    screen.getByTestId(`community-node-advisory-adoption-${node.id}`)
  ).getByRole('checkbox');
  expect(toggle).not.toBeChecked();
  expect(toggle).toHaveAccessibleDescription(/not asked for estimates/);

  await userEvent.click(toggle);
  expect(onChange).toHaveBeenCalledWith(node.id, true);
});

test('hides the toggle when the caller does not manage adoption', () => {
  const node = renderPanel({});
  expect(
    screen.queryByTestId(`community-node-advisory-adoption-${node.id}`)
  ).not.toBeInTheDocument();
});
