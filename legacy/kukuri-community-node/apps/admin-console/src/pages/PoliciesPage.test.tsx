import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { PoliciesPage } from './PoliciesPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    policies: vi.fn(),
    createPolicy: vi.fn(),
    updatePolicy: vi.fn(),
    publishPolicy: vi.fn(),
    makeCurrentPolicy: vi.fn()
  }
}));

describe('PoliciesPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.policies).mockResolvedValue([
      {
        policy_id: 'policy-1',
        policy_type: 'terms',
        version: '2026-01',
        locale: 'ja-JP',
        title: 'Terms of Service',
        content_md: 'initial content',
        content_hash: 'hash-1',
        published_at: null,
        effective_at: null,
        is_current: false
      }
    ]);
    vi.mocked(api.createPolicy).mockResolvedValue({
      policy_id: 'policy-2',
      policy_type: 'privacy',
      version: '2026-02',
      locale: 'ja-JP',
      title: 'Privacy Policy',
      content_md: 'privacy content',
      content_hash: 'hash-2',
      published_at: null,
      effective_at: null,
      is_current: false
    });
    vi.mocked(api.updatePolicy).mockResolvedValue({
      policy_id: 'policy-1',
      policy_type: 'terms',
      version: '2026-01',
      locale: 'ja-JP',
      title: 'Terms Updated',
      content_md: 'updated content',
      content_hash: 'hash-3',
      published_at: null,
      effective_at: null,
      is_current: false
    });
    vi.mocked(api.publishPolicy).mockResolvedValue({
      policy_id: 'policy-1',
      policy_type: 'terms',
      version: '2026-01',
      locale: 'ja-JP',
      title: 'Terms Updated',
      content_md: 'updated content',
      content_hash: 'hash-3',
      published_at: 1738809600,
      effective_at: 1738809600,
      is_current: false
    });
    vi.mocked(api.makeCurrentPolicy).mockResolvedValue({
      policy_id: 'policy-1',
      policy_type: 'terms',
      version: '2026-01',
      locale: 'ja-JP',
      title: 'Terms Updated',
      content_md: 'updated content',
      content_hash: 'hash-3',
      published_at: 1738809600,
      effective_at: 1738809600,
      is_current: true
    });
  });

  it('主要操作を送信し、一覧レイアウトを維持できる', async () => {
    renderWithQueryClient(<PoliciesPage />);

    expect(await screen.findByRole('heading', { name: 'Policies' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Policy List' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Create Policy' })).toBeInTheDocument();
    expect(await screen.findByText('Terms of Service')).toBeInTheDocument();
    expect(screen.getByText('Draft')).toBeInTheDocument();

    const createCard = screen.getByRole('heading', { name: 'Create Policy' }).closest('.card');
    expect(createCard).not.toBeNull();

    const createSelects = (createCard as HTMLElement).querySelectorAll('select');
    const createInputs = (createCard as HTMLElement).querySelectorAll('input');
    const createTextareas = (createCard as HTMLElement).querySelectorAll('textarea');
    const user = userEvent.setup();
    fireEvent.change(createSelects[0], { target: { value: 'privacy' } });
    fireEvent.change(createInputs[0], { target: { value: '2026-02' } });
    fireEvent.change(createInputs[1], { target: { value: 'ja-JP' } });
    fireEvent.change(createInputs[2], { target: { value: 'Privacy Policy' } });
    fireEvent.change(createTextareas[0], {
      target: { value: 'privacy content' }
    });
    await user.click(within(createCard as HTMLElement).getByRole('button', { name: 'Create' }));

    await waitFor(() => {
      expect(api.createPolicy).toHaveBeenCalledWith({
        policy_type: 'privacy',
        version: '2026-02',
        locale: 'ja-JP',
        title: 'Privacy Policy',
        content_md: 'privacy content'
      });
    });

    const policyCard = screen.getByText('Terms of Service').closest('.card');
    expect(policyCard).not.toBeNull();

    await user.click(within(policyCard as HTMLElement).getByRole('button', { name: 'Edit' }));
    expect(await screen.findByRole('heading', { name: 'Edit Policy' })).toBeInTheDocument();

    const editCard = screen.getByRole('heading', { name: 'Edit Policy' }).closest('.card');
    expect(editCard).not.toBeNull();
    const editInputs = (editCard as HTMLElement).querySelectorAll('input');
    const editTextareas = (editCard as HTMLElement).querySelectorAll('textarea');
    fireEvent.change(editInputs[2], { target: { value: 'Terms Updated' } });
    fireEvent.change(editTextareas[0], {
      target: { value: 'updated content' }
    });
    await user.click(within(editCard as HTMLElement).getByRole('button', { name: 'Update' }));

    await waitFor(() => {
      expect(api.updatePolicy).toHaveBeenCalledWith('policy-1', {
        title: 'Terms Updated',
        content_md: 'updated content'
      });
    });

    await user.click((await screen.findByRole('button', { name: 'Publish' })));
    await waitFor(() => {
      expect(api.publishPolicy).toHaveBeenCalledWith('policy-1');
    });

    await user.click((await screen.findByRole('button', { name: 'Make current' })));
    await waitFor(() => {
      expect(api.makeCurrentPolicy).toHaveBeenCalledWith('policy-1');
    });
  });
});
