import { render, screen, within } from '@testing-library/react';
import { expect, test } from 'vitest';

import { KeyboardPanel } from './KeyboardPanel';

// #964: 案内は実装済みのキーだけを列挙し、未対応キーを有効と表示しない。
test('keyboard panel lists assigned keys with their conditions and names unassigned keys', () => {
  render(<KeyboardPanel />);

  expect(screen.getByRole('heading', { name: 'Keyboard' })).toBeInTheDocument();
  const composer = screen.getByRole('region', { name: 'Composer' });
  const escRow = within(composer).getAllByRole('listitem')[0];
  expect(within(escRow).getByText('Esc', { selector: 'kbd' })).toBeInTheDocument();
  expect(escRow).toHaveTextContent('Close the composer');
  expect(escRow).toHaveTextContent('Works when: Focus is inside the composer');
  const sendRow = within(composer).getAllByRole('listitem')[1];
  expect(within(sendRow).getByText('Ctrl', { selector: 'kbd' })).toBeInTheDocument();
  expect(within(sendRow).getByText('Enter', { selector: 'kbd' })).toBeInTheDocument();
  expect(sendRow).toHaveTextContent('It sends exactly once');

  expect(screen.getByRole('region', { name: 'Screens and lists' })).toBeInTheDocument();
  expect(screen.getByText(/Ctrl\+K, Ctrl\+N\) are not assigned yet/)).toBeInTheDocument();
});
