import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { CommunityNodeTrustPriorityField } from './CommunityNodeTrustPriorityField';

// #1061: 信頼値の採用順位。選ばなければ折りたたまず、上位から採る順序を利用者が決める。
const FIRST = 'https://first.example';
const SECOND = 'https://second.example';

function renderField(priority: string[], onChange = vi.fn()) {
  render(
    <CommunityNodeTrustPriorityField
      configuredBaseUrls={[FIRST, SECOND]}
      priority={priority}
      onChange={onChange}
    />
  );
  return onChange;
}

test('explains that nothing is collapsed until a node is selected', () => {
  renderField([]);

  expect(screen.getByText(/no post is collapsed|折りたたみは行いません/)).toBeInTheDocument();
  for (const baseUrl of [FIRST, SECOND]) {
    expect(screen.getByTestId(`community-node-trust-priority-toggle-${baseUrl}`)).not.toBeChecked();
  }
});

test('selecting and reordering nodes reports the new priority', async () => {
  const onChange = renderField([FIRST]);

  const section = screen.getByTestId('community-node-trust-priority');
  expect(within(section).getByText('#1')).toBeInTheDocument();

  await userEvent.click(screen.getByTestId(`community-node-trust-priority-toggle-${SECOND}`));
  expect(onChange).toHaveBeenLastCalledWith([FIRST, SECOND]);

  onChange.mockClear();
  renderField([FIRST, SECOND], onChange);
  await userEvent.click(screen.getAllByRole('button', { name: `Move ${SECOND} up` })[0]);
  expect(onChange).toHaveBeenLastCalledWith([SECOND, FIRST]);
});

test('unselecting a node removes it from the priority', async () => {
  const onChange = renderField([FIRST, SECOND]);

  await userEvent.click(screen.getByTestId(`community-node-trust-priority-toggle-${FIRST}`));
  expect(onChange).toHaveBeenLastCalledWith([SECOND]);
});
