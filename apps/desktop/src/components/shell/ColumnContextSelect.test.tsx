import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { expect, test, vi } from 'vitest';

import { ColumnContextSelect } from './ColumnContextSelect';

test('exposes a compact labelled context switch and reports keyboard selection', async () => {
  const user = userEvent.setup();
  const onChange = vi.fn();
  render(
    <ColumnContextSelect
      label='Timeline topic'
      value='general'
      title='kukuri:topic:general'
      options={[
        { value: 'general', label: 'general' },
        { value: 'dev', label: 'Development topic with a long display name' },
      ]}
      onChange={onChange}
    />
  );

  const select = screen.getByRole('combobox', { name: 'Timeline topic' });
  expect(select).toHaveValue('general');
  expect(select).toHaveAttribute('title', 'kukuri:topic:general');
  await user.selectOptions(select, 'dev');
  expect(onChange).toHaveBeenCalledWith('dev');
});
