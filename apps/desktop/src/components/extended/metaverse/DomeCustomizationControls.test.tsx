import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, test, vi } from 'vitest';

import i18n from '@/i18n';
import { createDefaultDomeCustomization } from './DomeSceneModel';
import { DomeCustomizationControls } from './DomeCustomizationControls';

afterEach(async () => {
  await i18n.changeLanguage('en');
});

function renderControls(overrides: Partial<Parameters<typeof DomeCustomizationControls>[0]> = {}) {
  const props: Parameters<typeof DomeCustomizationControls>[0] = {
    customization: createDefaultDomeCustomization(),
    isOwner: true,
    pending: false,
    locale: 'en',
    onSave: vi.fn().mockResolvedValue(undefined),
    onImportTexture: vi.fn().mockResolvedValue({
      kind: 'texture',
      blob_hash: 'texture-1',
      mime_type: 'image/png',
      size_bytes: 1,
      name: 'wall.png',
    }),
    ...overrides,
  };
  return { ...render(<DomeCustomizationControls {...props} />), props };
}

describe('DomeCustomizationControls', () => {
  test('owner can edit a draft, save it, and see success feedback', async () => {
    const user = userEvent.setup();
    const { props } = renderControls();

    await user.selectOptions(screen.getByLabelText('Wall material'), 'wood');
    const gravity = screen.getByLabelText('Gravity (milli m/s²)');
    await user.clear(gravity);
    await user.type(gravity, '4900');
    await user.click(screen.getByRole('button', { name: 'Save Dome' }));

    await waitFor(() => expect(props.onSave).toHaveBeenCalledTimes(1));
    expect(props.onSave).toHaveBeenCalledWith(expect.objectContaining({
      surface: expect.objectContaining({ wall_material: 'wood' }),
      environment: expect.objectContaining({ gravity_milli: 4_900 }),
    }));
    expect(screen.getByText('Dome saved')).toBeInTheDocument();
  });

  test('cancel restores the current manifest draft', async () => {
    const user = userEvent.setup();
    renderControls();
    await user.selectOptions(screen.getByLabelText('Floor material'), 'metal');
    expect(screen.getByLabelText('Floor material')).toHaveValue('metal');
    await user.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(screen.getByLabelText('Floor material')).toHaveValue('stone');
  });

  test('invalid input is rejected before backend save', async () => {
    const user = userEvent.setup();
    const { props } = renderControls();
    const gravity = screen.getByLabelText('Gravity (milli m/s²)');
    await user.clear(gravity);
    await user.type(gravity, '999');
    await user.click(screen.getByRole('button', { name: 'Save Dome' }));
    expect(props.onSave).not.toHaveBeenCalled();
    expect(screen.getByText('Review values outside the supported range')).toBeInTheDocument();
  });

  test('non-owner receives a read-only summary without save action', () => {
    renderControls({ isOwner: false });
    expect(screen.getByText('Only the Dome owner can save these settings.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Save Dome' })).not.toBeInTheDocument();
  });

  test('pending and backend error states remain visible and non-destructive', async () => {
    const user = userEvent.setup();
    const onSave = vi.fn().mockRejectedValue(new Error('rejected'));
    const view = renderControls({ onSave });
    await user.click(screen.getByRole('button', { name: 'Save Dome' }));
    expect(await screen.findByText('Could not save Dome')).toBeInTheDocument();

    view.rerender(<DomeCustomizationControls {...view.props} pending />);
    expect(screen.getByLabelText('Wall material')).toBeDisabled();
    expect(screen.getByLabelText('Gravity (milli m/s²)')).toBeDisabled();
  });
});
