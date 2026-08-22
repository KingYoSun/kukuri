import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';

vi.mock('@/components/extended/metaverse/MetaverseRoomView', () => ({
  MetaverseRoomView: () => <div data-testid='metaverse-room-view'>Metaverse room preview</div>,
}));

import { VariableSpanColumnWorkspacePrototype } from './VariableSpanColumnWorkspacePrototype';

describe('VariableSpanColumnWorkspacePrototype', () => {
  it('announces Column position, active state, span, and pin state', async () => {
    const user = userEvent.setup();
    render(<VariableSpanColumnWorkspacePrototype scenario='single' />);

    const column = screen.getByRole('region', { name: 'Timeline Column' });
    expect(column).toHaveAttribute('aria-current', 'true');
    expect(column).toHaveAttribute('data-span', '1');
    expect(within(column).getByText('Column 1 of 1 · 1 span')).toHaveClass('sr-only');

    const pin = within(column).getByRole('button', { name: 'Pin Timeline' });
    expect(pin).toHaveAttribute('aria-pressed', 'false');
    await user.click(pin);
    expect(within(column).getByRole('button', { name: 'Unpin Timeline' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(within(column).getByText('Pinned')).toBeVisible();
  });

  it('keeps drag, menu reorder, primary action, and Control Center keyboard reachable', async () => {
    const user = userEvent.setup();
    render(<VariableSpanColumnWorkspacePrototype scenario='single' />);

    await user.tab();
    expect(screen.getByText('Skip to Columns')).toHaveFocus();
    await user.tab();
    expect(screen.getByRole('button', { name: 'Move Timeline' })).toHaveFocus();

    const menu = screen.getByRole('button', { name: 'Open Timeline menu' });
    await user.click(menu);
    expect(screen.getByRole('menuitem', { name: 'Move left' })).toBeVisible();
    expect(screen.getByRole('menuitem', { name: 'Move right' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Create post in Public · kukuri:topic:demo' })).toBeVisible();

    const trigger = screen.getByRole('button', { name: 'Open Control Center' });
    await user.click(trigger);
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    expect(screen.getByRole('complementary', { name: 'Control Center' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Find topic or channel' })).toBeVisible();
  });

  it('renders multi-span atomic surfaces and the mobile direct-jump affordance', () => {
    const { rerender } = render(<VariableSpanColumnWorkspacePrototype scenario='stream' />);
    expect(screen.getByRole('region', { name: 'Stream Column' })).toHaveAttribute('data-span', '2');

    rerender(<VariableSpanColumnWorkspacePrototype scenario='metaverse-4' />);
    expect(screen.getByRole('region', { name: 'Metaverse focused Column' })).toHaveAttribute(
      'data-span',
      '4'
    );

    rerender(<VariableSpanColumnWorkspacePrototype scenario='mobile' />);
    expect(screen.getByRole('navigation', { name: 'Column position' })).toHaveTextContent('1 / 3');
    expect(screen.getByRole('button', { name: 'Next Column' })).toBeEnabled();
  });

  it('exposes the reduced-motion review marker', () => {
    const { container } = render(
      <VariableSpanColumnWorkspacePrototype scenario='single' reducedMotion />
    );
    expect(container.querySelector('.variable-column-review')).toHaveAttribute(
      'data-reduced-motion',
      'reduce'
    );
  });
});
